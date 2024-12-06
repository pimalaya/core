use std::num::NonZeroU32;

use async_trait::async_trait;
use imap_client::imap_next::imap_types::{
    extensions::thread::Thread,
    search::SearchKey,
    sequence::{Sequence, SequenceSet},
};
use petgraph::{graphmap::DiGraphMap, Direction};
use tracing::{debug, instrument};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::ThreadEnvelopes;
use crate::{
    envelope::{list::ListEnvelopesOptions, SingleId, ThreadedEnvelope, ThreadedEnvelopes},
    imap::ImapContext,
    AnyResult,
};

#[derive(Clone, Debug)]
pub struct ThreadImapEnvelopes {
    ctx: ImapContext,
}

impl ThreadImapEnvelopes {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn ThreadEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn ThreadEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ThreadEnvelopes for ThreadImapEnvelopes {
    #[instrument(skip(self, opts))]
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        debug!(?opts, "thread options");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!(folder_encoded, "utf7 encoded folder");

        let folder_size = client.select_mailbox(folder_encoded).await?.exists.unwrap() as usize;
        debug!(folder_size, "folder size");

        if folder_size == 0 {
            return Ok(ThreadedEnvelopes::new(Default::default(), |_| {
                Default::default()
            }));
        }

        let threads = if let Some(query) = opts.query.as_ref() {
            let search_criteria = query.to_imap_search_criteria();
            client.thread_envelopes(search_criteria).await.unwrap()
        } else {
            client.thread_envelopes(Some(SearchKey::All)).await.unwrap()
        };

        let mut graph = DiGraphMap::<u32, u8>::new();

        for thread in threads {
            build_graph_from_thread(&mut graph, 0, 0, thread)
        }

        let uids: SequenceSet = graph
            .nodes()
            .filter_map(NonZeroU32::new)
            .map(Sequence::from)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let envelopes = client.fetch_envelopes_map(uids).await.unwrap();
        let envelopes = ThreadedEnvelopes::new(envelopes, move |envelopes| {
            let mut final_graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

            for (a, b, w) in graph.all_edges() {
                let eb = envelopes.get(&b.to_string()).unwrap();
                match envelopes.get(&a.to_string()) {
                    Some(ea) => {
                        final_graph.add_edge(ea.as_threaded(), eb.as_threaded(), *w);
                    }
                    None => {
                        let ea = ThreadedEnvelope {
                            id: "0",
                            message_id: "0",
                            subject: "",
                            from: "",
                            date: Default::default(),
                        };
                        final_graph.add_edge(ea, eb.as_threaded(), *w);
                    }
                }
            }

            final_graph
        });

        Ok(envelopes)
    }

    #[instrument(skip_all)]
    async fn thread_envelope(
        &self,
        folder: &str,
        id: SingleId,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!(folder_encoded, "utf7 encoded folder");

        let _folder_size = client.select_mailbox(folder_encoded).await?.exists.unwrap() as usize;
        debug!(folder_size = _folder_size, "folder size");

        let uid = id.parse::<u32>().unwrap();

        let threads = if let Some(query) = opts.query.as_ref() {
            let search_criteria = query.to_imap_search_criteria();
            client.thread_envelopes(search_criteria).await.unwrap()
        } else {
            client.thread_envelopes(Some(SearchKey::All)).await.unwrap()
        };

        let mut full_graph = DiGraphMap::<u32, u8>::new();

        for thread in threads {
            build_graph_from_thread(&mut full_graph, 0, 0, thread)
        }

        let mut graph = DiGraphMap::<u32, u8>::new();

        build_parents_graph(&full_graph, &mut graph, uid);
        build_children_graph(&full_graph, &mut graph, uid);

        let uids: SequenceSet = graph
            .nodes()
            .filter_map(NonZeroU32::new)
            .map(Sequence::from)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let envelopes = client.fetch_envelopes_map(uids).await.unwrap();
        let envelopes = ThreadedEnvelopes::new(envelopes, move |envelopes| {
            let mut final_graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

            for (a, b, w) in graph.all_edges() {
                let eb = envelopes.get(&b.to_string()).unwrap();
                match envelopes.get(&a.to_string()) {
                    Some(ea) => {
                        final_graph.add_edge(ea.as_threaded(), eb.as_threaded(), *w);
                    }
                    None => {
                        let ea = ThreadedEnvelope {
                            id: "0",
                            message_id: "0",
                            subject: "",
                            from: "",
                            date: Default::default(),
                        };
                        final_graph.add_edge(ea, eb.as_threaded(), *w);
                    }
                }
            }

            final_graph
        });

        Ok(envelopes)
    }
}

fn build_graph_from_thread(
    graph: &mut DiGraphMap<u32, u8>,
    mut parent_node: u32,
    mut weight: u8,
    thread: Thread,
) {
    match thread {
        Thread::Members { prefix, answers } => {
            for id in prefix {
                graph.add_edge(parent_node, id.into(), weight);
                parent_node = id.into();
                weight += 1;
            }

            if let Some(answers) = answers {
                for thread in answers {
                    build_graph_from_thread(graph, parent_node, weight, thread)
                }
            }
        }
        Thread::Nested { answers } => {
            for thread in answers {
                build_graph_from_thread(graph, parent_node, weight, thread)
            }
        }
    }
}

fn build_parents_graph(
    graph: &DiGraphMap<u32, u8>,
    parents_graph: &mut DiGraphMap<u32, u8>,
    cursor: u32,
) {
    for parent in graph.neighbors_directed(cursor, Direction::Incoming) {
        let weight = *graph.edge_weight(parent, cursor).unwrap();
        parents_graph.add_edge(parent, cursor, weight);
        build_parents_graph(graph, parents_graph, parent);
    }
}

fn build_children_graph(
    graph: &DiGraphMap<u32, u8>,
    children_graph: &mut DiGraphMap<u32, u8>,
    cursor: u32,
) {
    for child in graph.neighbors_directed(cursor, Direction::Outgoing) {
        let weight = *graph.edge_weight(cursor, child).unwrap();
        children_graph.add_edge(cursor, child, weight);
        build_children_graph(graph, children_graph, child);
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU32;

    use imap_client::imap_next::imap_types::{
        core::{Vec1, Vec2},
        extensions::thread::Thread,
    };
    use petgraph::graphmap::DiGraphMap;

    fn assert_thread_eq_graph(thread: Thread, expected_graph: DiGraphMap<u32, u8>) {
        let mut graph = DiGraphMap::new();
        super::build_graph_from_thread(&mut graph, 0, 0, thread);

        for (a, b, w) in expected_graph.all_edges() {
            let weight = graph.remove_edge(a, b);
            assert_eq!(Some(*w), weight, "edge {a} → {b} expects weight {w}");
        }

        assert_eq!(0, graph.all_edges().count(), "more edges than expected");
    }

    #[test]
    fn imap_codec_thread_1() {
        let thread = Thread::Members {
            prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
            answers: None,
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_2() {
        let thread = Thread::Members {
            prefix: Vec1::try_from(vec![
                NonZeroU32::new(1).unwrap(),
                NonZeroU32::new(2).unwrap(),
            ])
            .unwrap(),
            answers: None,
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(1, 2, 1);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_3() {
        let thread = Thread::Nested {
            answers: Vec2::try_from(vec![
                Thread::Members {
                    prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
                    answers: None,
                },
                Thread::Members {
                    prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                    answers: None,
                },
            ])
            .unwrap(),
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(0, 2, 0);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_4() {
        let thread = Thread::Members {
            prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
            answers: Some(
                Vec2::try_from(vec![
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                        answers: None,
                    },
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                        answers: None,
                    },
                ])
                .unwrap(),
            ),
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(1, 2, 1);
        graph.add_edge(1, 3, 1);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_5() {
        let thread = Thread::Members {
            prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
            answers: Some(
                Vec2::try_from(vec![
                    Thread::Members {
                        prefix: Vec1::try_from(vec![
                            NonZeroU32::new(2).unwrap(),
                            NonZeroU32::new(4).unwrap(),
                        ])
                        .unwrap(),
                        answers: None,
                    },
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                        answers: None,
                    },
                ])
                .unwrap(),
            ),
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(1, 2, 1);
        graph.add_edge(2, 4, 2);
        graph.add_edge(1, 3, 1);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_6() {
        let thread = Thread::Members {
            prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
            answers: Some(
                Vec2::try_from(vec![
                    Thread::Members {
                        prefix: Vec1::try_from(vec![
                            NonZeroU32::new(2).unwrap(),
                            NonZeroU32::new(4).unwrap(),
                        ])
                        .unwrap(),
                        answers: Some(
                            Vec2::try_from(vec![
                                Thread::Members {
                                    prefix: Vec1::from(NonZeroU32::new(5).unwrap()),
                                    answers: None,
                                },
                                Thread::Members {
                                    prefix: Vec1::from(NonZeroU32::new(6).unwrap()),
                                    answers: None,
                                },
                            ])
                            .unwrap(),
                        ),
                    },
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                        answers: None,
                    },
                ])
                .unwrap(),
            ),
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(1, 2, 1);
        graph.add_edge(2, 4, 2);
        graph.add_edge(4, 5, 3);
        graph.add_edge(4, 6, 3);
        graph.add_edge(1, 3, 1);

        assert_thread_eq_graph(thread, graph);
    }

    #[test]
    fn imap_codec_thread_7() {
        let thread = Thread::Members {
            prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
            answers: Some(
                Vec2::try_from(vec![
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                        answers: None,
                    },
                    Thread::Members {
                        prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                        answers: None,
                    },
                    Thread::Nested {
                        answers: Vec2::try_from(vec![
                            Thread::Nested {
                                answers: Vec2::try_from(vec![
                                    Thread::Members {
                                        prefix: Vec1::from(NonZeroU32::new(4).unwrap()),
                                        answers: None,
                                    },
                                    Thread::Members {
                                        prefix: Vec1::from(NonZeroU32::new(5).unwrap()),
                                        answers: None,
                                    },
                                ])
                                .unwrap(),
                            },
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(6).unwrap()),
                                answers: None,
                            },
                        ])
                        .unwrap(),
                    },
                ])
                .unwrap(),
            ),
        };

        let mut graph = DiGraphMap::new();
        graph.add_edge(0, 1, 0);
        graph.add_edge(1, 2, 1);
        graph.add_edge(1, 3, 1);
        graph.add_edge(1, 4, 1);
        graph.add_edge(1, 5, 1);
        graph.add_edge(1, 6, 1);

        assert_thread_eq_graph(thread, graph);
    }
}

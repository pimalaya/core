use std::num::NonZeroU32;

use async_trait::async_trait;
use imap_client::imap_flow::imap_codec::imap_types::{
    extensions::thread::Thread,
    sequence::{Sequence, SequenceSet},
};
use petgraph::graphmap::DiGraphMap;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::ThreadEnvelopes;
use crate::{
    debug,
    envelope::{list::ListEnvelopesOptions, ThreadedEnvelopes},
    imap::ImapContextSync,
    AnyResult,
};

#[derive(Clone, Debug)]
pub struct ThreadImapEnvelopes {
    ctx: ImapContextSync,
}

impl ThreadImapEnvelopes {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn ThreadEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ThreadEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ThreadEnvelopes for ThreadImapEnvelopes {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, opts)))]
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        debug!(?opts, "thread options");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!(folder_encoded, "utf7 encoded folder");

        let folder_size = ctx.select_mailbox(folder_encoded).await?.exists.unwrap() as usize;
        debug!(folder_size, "folder size");

        if folder_size == 0 {
            return Ok(ThreadedEnvelopes::new(Default::default(), |_| {
                Default::default()
            }));
        }

        // let envelopes = if let Some(query) = opts.query.as_ref() {
        //     let search_criteria = query.to_imap_search_criteria();

        //     let mut envelopes = ctx.thread_envelopes(search_criteria).await.unwrap();

        //     apply_pagination(&mut envelopes, opts.page, opts.page_size)?;

        //     envelopes
        // } else {
        //     let seq = build_sequence(opts.page, opts.page_size, folder_size)?;
        //     let mut envelopes = ctx.fetch_envelopes_by_sequence(seq.into()).await?;
        //     envelopes.sort_by(|a, b| b.date.cmp(&a.date));
        //     envelopes
        // };

        let threads = ctx
            .thread_envelopes(Some(
                imap_client::imap_flow::imap_codec::imap_types::search::SearchKey::All,
            ))
            .await
            .unwrap();

        // apply_pagination(&mut envelopes, opts.page, opts.page_size)?;

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

        let envelopes = ctx.fetch_envelopes_map(uids).await.unwrap();
        let envelopes = ThreadedEnvelopes::new(envelopes, move |envelopes| {
            let mut final_graph = DiGraphMap::<&str, u8>::new();

            for (a, b, w) in graph.all_edges() {
                let eb = envelopes.get(&b.to_string()).unwrap();
                match envelopes.get(&a.to_string()) {
                    Some(ea) => {
                        final_graph.add_edge(ea.message_id.as_str(), eb.message_id.as_str(), *w);
                    }
                    None => {
                        final_graph.add_edge("root", eb.message_id.as_str(), *w);
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

#[cfg(test)]
mod test {
    use std::num::NonZeroU32;

    use imap_client::imap_flow::imap_codec::imap_types::{
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

use std::sync::Arc;

use async_trait::async_trait;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use unicode_segmentation::UnicodeSegmentation;

/// A validated immutable plan for translating long text without changing its source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkPlan {
    chunks: Vec<TextChunk>,
}

impl ChunkPlan {
    #[must_use]
    pub fn chunks(&self) -> &[TextChunk] {
        &self.chunks
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    #[must_use]
    pub fn reconstruct_source(&self) -> String {
        self.chunks.iter().map(TextChunk::source).collect()
    }
}

/// One contiguous, source-ordered text range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextChunk {
    index: usize,
    source: String,
    source_start: usize,
    source_end: usize,
    paragraph_id: usize,
}

impl TextChunk {
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn source_range(&self) -> std::ops::Range<usize> {
        self.source_start..self.source_end
    }

    #[must_use]
    pub const fn paragraph_id(&self) -> usize {
        self.paragraph_id
    }
}

/// Deterministically partitions text at paragraph boundaries before any provider request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkPlanner {
    maximum_graphemes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkPlanError {
    ZeroMaximum,
}

/// Executes one complete chunk without exposing a provider protocol to the planner.
#[async_trait]
pub trait ChunkTranslator: Send + Sync + 'static {
    type Output: Send + 'static;
    type Error: Send + 'static;

    async fn translate(
        &self,
        chunk: TextChunk,
        cancellation: CancellationToken,
    ) -> Result<Self::Output, Self::Error>;
}

/// Failure from the all-or-nothing long-text executor.
#[derive(Debug)]
pub enum ChunkExecutionError<E> {
    Translation(E),
    Cancelled,
    Join,
}

type ChunkTaskResult<T> =
    Result<(usize, <T as ChunkTranslator>::Output), <T as ChunkTranslator>::Error>;

/// Runs a complete plan with at most two active children and returns results in source order.
///
/// # Errors
///
/// Returns cancellation, task-join, or the first child translation error. No partial result is returned.
pub async fn execute_in_order<T>(
    plan: &ChunkPlan,
    translator: Arc<T>,
    cancellation: CancellationToken,
) -> Result<Vec<T::Output>, ChunkExecutionError<T::Error>>
where
    T: ChunkTranslator,
{
    let child_cancellation = cancellation.child_token();
    let mut pending = plan.chunks.iter().cloned();
    let mut running = JoinSet::new();
    let mut completed = Vec::with_capacity(plan.len());

    while running.len() < 2 {
        let Some(chunk) = pending.next() else { break };
        spawn_chunk(
            &mut running,
            Arc::clone(&translator),
            chunk,
            child_cancellation.clone(),
        );
    }
    while let Some(joined) = tokio::select! {
        joined = running.join_next() => joined,
        () = cancellation.cancelled() => {
            child_cancellation.cancel();
            running.abort_all();
            return Err(ChunkExecutionError::Cancelled);
        }
    } {
        match joined {
            Ok(Ok((index, result))) => {
                completed.push((index, result));
                if let Some(chunk) = pending.next() {
                    spawn_chunk(
                        &mut running,
                        Arc::clone(&translator),
                        chunk,
                        child_cancellation.clone(),
                    );
                }
            }
            Ok(Err(error)) => {
                child_cancellation.cancel();
                running.abort_all();
                return Err(ChunkExecutionError::Translation(error));
            }
            Err(_) => {
                child_cancellation.cancel();
                running.abort_all();
                return Err(ChunkExecutionError::Join);
            }
        }
    }
    completed.sort_by_key(|(index, _)| *index);
    Ok(completed.into_iter().map(|(_, result)| result).collect())
}

fn spawn_chunk<T>(
    running: &mut JoinSet<ChunkTaskResult<T>>,
    translator: Arc<T>,
    chunk: TextChunk,
    cancellation: CancellationToken,
) where
    T: ChunkTranslator,
{
    running.spawn(async move {
        let index = chunk.index;
        translator
            .translate(chunk, cancellation)
            .await
            .map(|result| (index, result))
    });
}

impl ChunkPlanner {
    /// Creates a planner with a non-zero per-request grapheme budget.
    ///
    /// # Errors
    ///
    /// Returns [`ChunkPlanError::ZeroMaximum`] when the budget is zero.
    pub const fn new(maximum_graphemes: usize) -> Result<Self, ChunkPlanError> {
        if maximum_graphemes == 0 {
            return Err(ChunkPlanError::ZeroMaximum);
        }
        Ok(Self { maximum_graphemes })
    }

    /// Plans contiguous chunks while preserving every source grapheme and separator exactly.
    ///
    /// # Errors
    ///
    /// Returns an error only when this planner's invariant is invalid.
    pub fn plan(&self, source: &str) -> Result<ChunkPlan, ChunkPlanError> {
        let mut chunks = Vec::new();
        let mut text_start = 0;
        let mut fenced_block_start = None;
        let mut offset = 0;
        for line in source.split_inclusive('\n') {
            let line_end = offset + line.len();
            if line.trim_end_matches('\n').starts_with("```") {
                if let Some(block_start) = fenced_block_start.take() {
                    self.append_range(source, block_start, line_end, &mut chunks, true);
                    text_start = line_end;
                } else {
                    self.append_range(source, text_start, offset, &mut chunks, false);
                    fenced_block_start = Some(offset);
                }
            } else if fenced_block_start.is_none() && is_list_item(line) {
                self.append_range(source, text_start, offset, &mut chunks, false);
                self.append_range(source, offset, line_end, &mut chunks, true);
                text_start = line_end;
            }
            offset = line_end;
        }
        if let Some(block_start) = fenced_block_start {
            self.append_range(source, block_start, source.len(), &mut chunks, true);
        } else {
            self.append_range(source, text_start, source.len(), &mut chunks, false);
        }
        Ok(ChunkPlan { chunks })
    }

    fn append_range(
        self,
        source: &str,
        start: usize,
        end: usize,
        chunks: &mut Vec<TextChunk>,
        atomic: bool,
    ) {
        let range = &source[start..end];
        if atomic || range.graphemes(true).count() <= self.maximum_graphemes || range.is_empty() {
            if !range.is_empty() {
                push_chunk(
                    chunks,
                    range.to_owned(),
                    start,
                    end,
                    paragraph_id_at(source, start),
                );
            }
            return;
        }
        let sentences = sentence_ranges(range);
        if sentences.len() > 1 {
            for (offset, sentence) in sentences {
                self.append_graphemes(source, start + offset, sentence, chunks);
            }
            return;
        }
        self.append_graphemes(source, start, range, chunks);
    }

    fn append_graphemes(
        self,
        source: &str,
        start: usize,
        range: &str,
        chunks: &mut Vec<TextChunk>,
    ) {
        let mut current = String::new();
        let mut current_start = start;
        let mut current_end = start;
        for grapheme in range.graphemes(true) {
            if current.graphemes(true).count() == self.maximum_graphemes {
                push_chunk(
                    chunks,
                    std::mem::take(&mut current),
                    current_start,
                    current_end,
                    paragraph_id_at(source, current_start),
                );
                current_start = current_end;
            }
            current.push_str(grapheme);
            current_end += grapheme.len();
        }
        if !current.is_empty() {
            push_chunk(
                chunks,
                current,
                current_start,
                current_end,
                paragraph_id_at(source, current_start),
            );
        }
    }
}

fn push_chunk(
    chunks: &mut Vec<TextChunk>,
    source: String,
    source_start: usize,
    source_end: usize,
    paragraph_id: usize,
) {
    chunks.push(TextChunk {
        index: chunks.len(),
        source,
        source_start,
        source_end,
        paragraph_id,
    });
}

fn paragraph_id_at(source: &str, offset: usize) -> usize {
    source[..offset].matches("\n\n").count()
}

fn sentence_ranges(source: &str) -> Vec<(usize, &str)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut previous_was_terminal = false;
    for (offset, character) in source.char_indices() {
        if character.is_whitespace() && previous_was_terminal {
            if character == '\n' && source[offset + character.len_utf8()..].starts_with('\n') {
                continue;
            }
            let end = offset + character.len_utf8();
            ranges.push((start, &source[start..end]));
            start = end;
            previous_was_terminal = false;
        } else {
            previous_was_terminal = matches!(character, '.' | '!' | '?' | '。' | '！' | '？');
        }
    }
    if start < source.len() {
        ranges.push((start, &source[start..]));
    }
    ranges
}

fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    matches!(trimmed.as_bytes(), [b'-' | b'*' | b'+', b' ', ..])
        || trimmed.split_once('.').is_some_and(|(marker, remainder)| {
            !marker.is_empty()
                && marker.bytes().all(|byte| byte.is_ascii_digit())
                && remainder.starts_with(' ')
        })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use tokio::time::{Duration, sleep};
    use tokio_util::sync::CancellationToken;

    #[test]
    fn chunk_plan_preserves_paragraph_boundaries_and_exact_source() {
        let plan = super::ChunkPlanner::new(24).expect("planner");
        let source = "First paragraph.\n\nSecond paragraph.";

        let chunks = plan.plan(source).expect("plan");

        assert_eq!(chunks.reconstruct_source(), source);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn chunk_plan_assigns_contiguous_indices_source_ranges_and_paragraph_ids() {
        let plan = super::ChunkPlanner::new(8).expect("planner");
        let chunks = plan.plan("Alpha.\n\nBeta.").expect("plan");

        assert_eq!(chunks.chunks()[0].index(), 0);
        assert_eq!(chunks.chunks()[0].source_range(), 0..8);
        assert_eq!(chunks.chunks()[0].paragraph_id(), 0);
        assert_eq!(chunks.chunks()[1].index(), 1);
        assert_eq!(chunks.chunks()[1].source_range(), 8..13);
        assert_eq!(chunks.chunks()[1].paragraph_id(), 1);
    }

    #[test]
    fn chunk_plan_reconstructs_representative_unicode_and_structured_sources_exactly() {
        let plan = super::ChunkPlanner::new(3).expect("planner");
        for source in [
            "👨‍👩‍👧‍👦 café\n\n第二段。",
            "- 一\n- two\n\n```text\n😀😀😀😀\n```\n",
            "A sentence. B sentence. C sentence.",
        ] {
            let chunks = plan.plan(source).expect("plan");
            assert_eq!(chunks.reconstruct_source(), source);
            assert!(chunks.chunks().iter().enumerate().all(|(index, chunk)| {
                chunk.index() == index && &source[chunk.source_range()] == chunk.source()
            }));
        }
    }

    #[test]
    fn chunk_plan_keeps_fenced_code_blocks_atomic_even_when_they_exceed_the_budget() {
        let plan = super::ChunkPlanner::new(8).expect("planner");
        let source = "Intro.\n\n```rust\nlet greeting = \"hello\";\n```\n\nOutro.";

        let chunks = plan.plan(source).expect("plan");

        assert_eq!(chunks.reconstruct_source(), source);
        assert!(
            chunks
                .chunks()
                .iter()
                .any(|chunk| chunk.source() == "```rust\nlet greeting = \"hello\";\n```\n")
        );
    }

    #[test]
    fn chunk_plan_keeps_each_list_item_atomic() {
        let plan = super::ChunkPlanner::new(10).expect("planner");
        let source = "- first item is deliberately long\n- second item\n";

        let chunks = plan.plan(source).expect("plan");

        assert_eq!(chunks.reconstruct_source(), source);
        assert!(
            chunks
                .chunks()
                .iter()
                .any(|chunk| chunk.source() == "- first item is deliberately long\n")
        );
    }

    #[test]
    fn chunk_plan_splits_an_oversized_paragraph_at_sentence_boundaries_before_graphemes() {
        let plan = super::ChunkPlanner::new(20).expect("planner");
        let source = "First sentence. Second sentence.";

        let chunks = plan.plan(source).expect("plan");

        assert_eq!(chunks.reconstruct_source(), source);
        assert_eq!(
            chunks
                .chunks()
                .iter()
                .map(super::TextChunk::source)
                .collect::<Vec<_>>(),
            vec!["First sentence. ", "Second sentence."]
        );
    }

    struct OutOfOrderTranslator;

    #[async_trait]
    impl super::ChunkTranslator for OutOfOrderTranslator {
        type Output = usize;
        type Error = ();

        async fn translate(
            &self,
            chunk: super::TextChunk,
            _: CancellationToken,
        ) -> Result<Self::Output, Self::Error> {
            sleep(Duration::from_millis((2 - chunk.index()) as u64 * 5)).await;
            Ok(chunk.index())
        }
    }

    #[tokio::test]
    async fn executor_returns_completed_chunks_in_source_order_after_out_of_order_completion() {
        let plan = super::ChunkPlanner::new(8)
            .expect("planner")
            .plan("- one\n- two\n- three\n")
            .expect("plan");

        let results = super::execute_in_order(
            &plan,
            Arc::new(OutOfOrderTranslator),
            CancellationToken::new(),
        )
        .await
        .expect("complete result");

        assert_eq!(results, vec![0, 1, 2]);
    }

    struct ConcurrencyTrackingTranslator {
        active: AtomicUsize,
        maximum: AtomicUsize,
    }

    #[async_trait]
    impl super::ChunkTranslator for ConcurrencyTrackingTranslator {
        type Output = usize;
        type Error = ();

        async fn translate(
            &self,
            chunk: super::TextChunk,
            _: CancellationToken,
        ) -> Result<Self::Output, Self::Error> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.maximum.fetch_max(active, Ordering::SeqCst);
            sleep(Duration::from_millis(10)).await;
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(chunk.index())
        }
    }

    #[tokio::test]
    async fn executor_never_runs_more_than_two_children_at_once() {
        let plan = super::ChunkPlanner::new(8)
            .expect("planner")
            .plan("- one\n- two\n- three\n- four\n")
            .expect("plan");
        let translator = Arc::new(ConcurrencyTrackingTranslator {
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
        });

        super::execute_in_order(&plan, Arc::clone(&translator), CancellationToken::new())
            .await
            .expect("complete result");

        assert_eq!(translator.maximum.load(Ordering::SeqCst), 2);
    }

    struct StartCountingTranslator(AtomicUsize);

    #[async_trait]
    impl super::ChunkTranslator for StartCountingTranslator {
        type Output = ();
        type Error = ();

        async fn translate(
            &self,
            _: super::TextChunk,
            _: CancellationToken,
        ) -> Result<Self::Output, Self::Error> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn executor_does_not_start_children_when_the_parent_is_already_cancelled() {
        let plan = super::ChunkPlanner::new(8)
            .expect("planner")
            .plan("- one\n- two\n")
            .expect("plan");
        let translator = Arc::new(StartCountingTranslator(AtomicUsize::new(0)));
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let result = super::execute_in_order(&plan, Arc::clone(&translator), cancellation).await;

        assert!(matches!(result, Err(super::ChunkExecutionError::Cancelled)));
        assert_eq!(translator.0.load(Ordering::SeqCst), 0);
    }

    struct FailingTranslator;

    #[async_trait]
    impl super::ChunkTranslator for FailingTranslator {
        type Output = usize;
        type Error = &'static str;

        async fn translate(
            &self,
            chunk: super::TextChunk,
            _: CancellationToken,
        ) -> Result<Self::Output, Self::Error> {
            if chunk.index() == 1 {
                Err("provider failed")
            } else {
                sleep(Duration::from_millis(20)).await;
                Ok(chunk.index())
            }
        }
    }

    #[tokio::test]
    async fn executor_discards_completed_chunks_when_any_child_fails() {
        let plan = super::ChunkPlanner::new(8)
            .expect("planner")
            .plan("- one\n- two\n- three\n")
            .expect("plan");

        let result =
            super::execute_in_order(&plan, Arc::new(FailingTranslator), CancellationToken::new())
                .await;

        assert!(matches!(
            result,
            Err(super::ChunkExecutionError::Translation("provider failed"))
        ));
    }
}

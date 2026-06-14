use aftercode_core::episode::LearningTopic;

fn score(t: &LearningTopic) -> u8 {
    let p = match t.priority.as_str() {
        "high" => 3,
        "medium" => 2,
        _ => 1,
    };
    let e = if t.evidence.is_empty() { 0 } else { 1 };
    p + e
}

/// Sort topics by priority+evidence desc, keep at most `max`.
pub fn rank(mut topics: Vec<LearningTopic>, max: usize) -> Vec<LearningTopic> {
    topics.sort_by_key(|t| std::cmp::Reverse(score(t)));
    topics.truncate(max);
    topics
}

#[cfg(test)]
mod tests {
    use super::*;
    fn topic(pri: &str, ev: bool) -> LearningTopic {
        LearningTopic {
            title: "t".into(),
            summary: "s".into(),
            evidence: if ev { vec!["x".into()] } else { vec![] },
            knowledge_gap: "g".into(),
            difficulty: "intermediate".into(),
            priority: pri.into(),
        }
    }
    #[test]
    fn high_priority_with_evidence_ranks_first() {
        let out = rank(vec![topic("low", false), topic("high", true)], 5);
        assert_eq!(out[0].priority, "high");
    }
    #[test]
    fn truncates_to_max() {
        let out = rank(
            vec![
                topic("high", true),
                topic("high", true),
                topic("high", true),
            ],
            2,
        );
        assert_eq!(out.len(), 2);
    }
}

use std::ops::RangeInclusive;

pub fn expand_context(
    hit_lines: &[usize],
    context_lines: usize,
    total_lines: usize,
) -> Vec<RangeInclusive<usize>> {
    if hit_lines.is_empty() || total_lines == 0 {
        return Vec::new();
    }

    let max_idx = total_lines - 1;
    hit_lines
        .iter()
        .map(|&line| {
            let start = line.saturating_sub(context_lines);
            let end = (line + context_lines).min(max_idx);
            start..=end
        })
        .collect()
}

pub fn merge_ranges(ranges: &mut Vec<RangeInclusive<usize>>) {
    if ranges.len() <= 1 {
        return;
    }
    ranges.sort_by_key(|r| *r.start());

    let mut merged: Vec<RangeInclusive<usize>> = Vec::with_capacity(ranges.len());
    let mut current = ranges[0].clone();

    for r in &ranges[1..] {
        if *r.start() <= *current.end() + 1 {
            current = *current.start()..=(*current.end()).max(*r.end());
        } else {
            merged.push(current);
            current = r.clone();
        }
    }
    merged.push(current);
    *ranges = merged;
}

pub fn ranges_to_line_set(ranges: &[RangeInclusive<usize>]) -> Vec<usize> {
    let mut lines = Vec::new();
    for r in ranges {
        for i in r.clone() {
            lines.push(i);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_basic() {
        let ranges = expand_context(&[5, 10], 2, 20);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], 3..=7);
        assert_eq!(ranges[1], 8..=12);
    }

    #[test]
    fn expand_clamps_to_zero() {
        let ranges = expand_context(&[1], 3, 100);
        assert_eq!(ranges[0], 0..=4);
    }

    #[test]
    fn expand_clamps_to_max() {
        let ranges = expand_context(&[98], 3, 100);
        assert_eq!(ranges[0], 95..=99);
    }

    #[test]
    fn merge_overlapping() {
        let mut ranges = vec![3..=7, 6..=12, 20..=25];
        merge_ranges(&mut ranges);
        assert_eq!(ranges, vec![3..=12, 20..=25]);
    }

    #[test]
    fn merge_adjacent() {
        let mut ranges = vec![3..=7, 8..=12];
        merge_ranges(&mut ranges);
        assert_eq!(ranges, vec![3..=12]);
    }

    #[test]
    fn merge_no_overlap() {
        let mut ranges = vec![1..=3, 10..=12];
        merge_ranges(&mut ranges);
        assert_eq!(ranges, vec![1..=3, 10..=12]);
    }

    #[test]
    fn ranges_to_lines() {
        let lines = ranges_to_line_set(&[2..=4, 8..=9]);
        assert_eq!(lines, vec![2, 3, 4, 8, 9]);
    }

    #[test]
    fn empty_input() {
        assert!(expand_context(&[], 2, 100).is_empty());
        assert!(expand_context(&[5], 2, 0).is_empty());
    }
}

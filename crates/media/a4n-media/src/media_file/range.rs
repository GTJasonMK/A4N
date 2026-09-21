pub fn requested_byte_range(value: &str, size: u64) -> Option<Result<(u64, u64), ()>> {
    let value = value.trim();
    let range = value.strip_prefix("bytes=")?;
    if range.contains(',') {
        return Some(Err(()));
    }
    let Some((start, end)) = range.split_once('-') else {
        return Some(Err(()));
    };
    if size == 0 {
        return Some(Err(()));
    }
    if start.is_empty() {
        let suffix_len = match end.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => return Some(Err(())),
        };
        let len = suffix_len.min(size);
        return Some(Ok((size - len, size - 1)));
    }

    let start = match start.parse::<u64>() {
        Ok(value) => value,
        Err(_) => return Some(Err(())),
    };
    let end = if end.is_empty() {
        size - 1
    } else {
        match end.parse::<u64>() {
            Ok(value) => value,
            Err(_) => return Some(Err(())),
        }
    };
    if start >= size || end < start {
        Some(Err(()))
    } else {
        Some(Ok((start, end.min(size - 1))))
    }
}

pub fn requested_byte_range_header<E>(
    value: Option<Result<&str, E>>,
    size: u64,
) -> Option<Result<(u64, u64), ()>> {
    match value {
        Some(Ok(value)) => requested_byte_range(value, size),
        Some(Err(_)) => Some(Err(())),
        None => None,
    }
}

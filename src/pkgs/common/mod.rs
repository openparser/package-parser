use anyhow::{anyhow, Result};

pub mod model;

pub fn decode_string(buffer: &[u8]) -> Result<String> {
    let mut failed_encodings = vec![];

    let (s, _, has_error) = encoding_rs::UTF_8.decode(buffer);
    if has_error {
        failed_encodings.push("UTF-8");
    } else {
        return Ok(s.to_string());
    }

    let (s, _, has_error) = encoding_rs::UTF_16LE.decode(buffer);
    if has_error {
        failed_encodings.push("UTF-16LE");
    } else {
        log::warn!("UTF-16LE encoding detected. This is not recommended.");
        return Ok(s.to_string());
    }

    let (s, _, has_error) = encoding_rs::UTF_16BE.decode(buffer);
    if has_error {
        failed_encodings.push("UTF-16BE");
    } else {
        log::warn!("UTF-16BE encoding detected. This is not recommended.");
        return Ok(s.to_string());
    }

    let (s, _, has_error) = encoding_rs::GB18030.decode(buffer);
    if has_error {
        failed_encodings.push("GB18030");
    } else {
        log::warn!("GB18030 encoding detected. This is not recommended.");
        return Ok(s.to_string());
    }

    let (s, _, has_error) = encoding_rs::SHIFT_JIS.decode(buffer);
    if has_error {
        failed_encodings.push("SHIFT-JIS");
    } else {
        log::warn!("SHIFT-JIS encoding detected. This is not recommended.");
        return Ok(s.to_string());
    }

    Err(anyhow!(
        "Failed to decode string. Tried encodings: {}",
        failed_encodings.join(", ")
    ))
}

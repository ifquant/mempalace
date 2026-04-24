use crate::error::{MempalaceError, Result};
use crate::model::DrawerInput;
use crate::storage::sqlite::DrawerRecord;
use chrono::Utc;

pub fn build_manual_drawer(
    wing: &str,
    room: &str,
    content: &str,
    source_file: Option<&str>,
    added_by: Option<&str>,
) -> Result<DrawerInput> {
    let sanitized_wing = sanitize_name(wing, "wing")?;
    let sanitized_room = sanitize_name(room, "room")?;
    let sanitized_content = sanitize_content(content)?;
    let sanitized_added_by = sanitize_name(added_by.unwrap_or("mcp"), "added_by")?;
    let normalized_source_file = source_file.unwrap_or_default().trim().to_string();
    let content_preview = sanitized_content
        .char_indices()
        .nth(100)
        .map(|(idx, _)| &sanitized_content[..idx])
        .unwrap_or(&sanitized_content);
    let wing_slug = identifier_fragment(&sanitized_wing);
    let room_slug = identifier_fragment(&sanitized_room);
    let drawer_id = format!(
        "drawer_{}_{}_{}",
        wing_slug,
        room_slug,
        &blake3::hash(format!("{sanitized_wing}|{sanitized_room}|{content_preview}").as_bytes())
            .to_hex()
            .to_string()[..24]
    );

    Ok(DrawerInput {
        id: drawer_id.clone(),
        wing: sanitized_wing,
        room: sanitized_room,
        source_file: normalized_source_file.clone(),
        source_path: if normalized_source_file.is_empty() {
            format!("mcp://{wing_slug}/{room_slug}/{drawer_id}")
        } else {
            normalized_source_file
        },
        source_hash: blake3::hash(sanitized_content.as_bytes())
            .to_hex()
            .to_string(),
        source_mtime: None,
        chunk_index: 0,
        added_by: sanitized_added_by,
        filed_at: Utc::now().to_rfc3339(),
        ingest_mode: "mcp".to_string(),
        extract_mode: "manual".to_string(),
        text: sanitized_content,
    })
}

pub fn drawer_input_from_record(record: &DrawerRecord) -> DrawerInput {
    DrawerInput {
        id: record.id.clone(),
        wing: record.wing.clone(),
        room: record.room.clone(),
        source_file: record.source_file.clone(),
        source_path: record.source_path.clone(),
        source_hash: record.source_hash.clone(),
        source_mtime: record.source_mtime,
        chunk_index: record.chunk_index,
        added_by: record.added_by.clone(),
        filed_at: record.filed_at.clone(),
        ingest_mode: record.ingest_mode.clone(),
        extract_mode: record.extract_mode.clone(),
        text: record.text.clone(),
    }
}

pub fn sanitize_name(value: &str, field_name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} must be a non-empty string"
        )));
    }

    if trimmed.len() > 128 {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} exceeds maximum length of 128 characters"
        )));
    }

    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains invalid path characters"
        )));
    }

    if trimmed.contains('\0') {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains null bytes"
        )));
    }

    let valid = trimmed.chars().enumerate().all(|(idx, ch)| {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '_' | ' ' | '.' | '\'' | '-');
        if !allowed {
            return false;
        }
        if (idx == 0 || idx == trimmed.len() - 1) && !ch.is_ascii_alphanumeric() {
            return false;
        }
        true
    });

    if !valid {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains invalid characters"
        )));
    }

    Ok(trimmed.to_string())
}

fn sanitize_content(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MempalaceError::InvalidArgument(
            "content cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 100_000 {
        return Err(MempalaceError::InvalidArgument(
            "content exceeds maximum length of 100000 characters".to_string(),
        ));
    }
    if trimmed.contains('\0') {
        return Err(MempalaceError::InvalidArgument(
            "content contains null bytes".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn identifier_fragment(value: &str) -> String {
    let fragment = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let fragment = fragment.trim_matches('-').to_string();
    if fragment.is_empty() {
        "item".to_string()
    } else {
        fragment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_manual_drawer_matches_python_style_id_and_source_path() {
        let drawer = build_manual_drawer(
            "project alpha",
            "backend",
            "Planning notes about rollout",
            None,
            Some("tester"),
        )
        .unwrap();

        assert!(drawer.id.starts_with("drawer_project-alpha_backend_"));
        assert_eq!(
            drawer.source_path,
            format!("mcp://project-alpha/backend/{}", drawer.id)
        );
        assert_eq!(drawer.ingest_mode, "mcp");
        assert_eq!(drawer.extract_mode, "manual");
    }

    #[test]
    fn build_manual_drawer_rejects_invalid_names_and_empty_content() {
        assert!(build_manual_drawer("..", "room", "content", None, None).is_err());
        assert!(build_manual_drawer("wing", "room", "   ", None, None).is_err());
    }
}

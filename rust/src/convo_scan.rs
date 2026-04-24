#[path = "convo_scan_include.rs"]
mod include;
#[path = "convo_scan_walk.rs"]
mod walk;

pub use walk::scan_convo_files;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::include::{is_exact_force_include, is_force_include, normalize_include_paths};
    use super::walk::should_skip_convo_file;

    #[test]
    fn include_paths_normalize_slashes_and_trim_edges() {
        let include = normalize_include_paths(&[
            " notes/session.jsonl ".to_string(),
            "/logs\\chat.txt/".to_string(),
        ]);

        assert!(include.contains("notes/session.jsonl"));
        assert!(include.contains("logs/chat.txt"));
    }

    #[test]
    fn force_include_matches_exact_file_and_parent_prefixes() {
        let root = Path::new("/tmp/project");
        let file_include = normalize_include_paths(&["logs/chat.txt".to_string()]);
        let dir_include = normalize_include_paths(&["logs".to_string()]);

        assert!(is_exact_force_include(
            &root.join("logs/chat.txt"),
            root,
            &file_include
        ));
        assert!(is_force_include(
            &root.join("logs/chat.txt"),
            root,
            &file_include
        ));
        assert!(is_force_include(&root.join("logs"), root, &dir_include));
        assert!(is_force_include(
            &root.join("logs/chat.txt"),
            root,
            &dir_include
        ));
        assert!(!is_force_include(&root.join("notes"), root, &file_include));
    }

    #[test]
    fn convo_scan_skips_meta_json_but_keeps_supported_exports() {
        assert!(should_skip_convo_file(Path::new("session.meta.json")));
        assert!(!should_skip_convo_file(Path::new("session.json")));
        assert!(!should_skip_convo_file(Path::new("session.JSONL")));
        assert!(should_skip_convo_file(Path::new("session.csv")));
    }
}

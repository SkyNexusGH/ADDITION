//! Every trainer shipped with the app must parse and validate.

use addition_engine::Trainer;

#[test]
fn bundled_trainers_are_valid() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/trainers");
    let mut n = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).unwrap();
            let t = Trainer::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(path.file_stem().unwrap().to_str(), Some(t.id.as_str()), "file name must match id");
            n += 1;
        }
    }
    assert!(n > 0);
}

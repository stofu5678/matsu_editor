use super::*;

fn profile(text: &str) -> Profile {
    // A file with the BOM is UTF-8, as the one UME Editor writes.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

#[test]
fn a_new_file_has_no_macros_and_the_timer_of_sakura() {
    let table = RegisteredMacros::from_profile(&profile(""));
    assert_eq!(table, RegisteredMacros::default());
    assert_eq!(table.macros.len(), 50);
    assert!(table.macros.iter().all(|m| !m.is_enabled()));
    assert_eq!(table.cancel_timer, 10);
    assert_eq!(
        (table.on_opened, table.on_type_changed, table.on_save),
        (None, None, None)
    );
}

#[test]
fn rows_are_read_with_the_keys_of_sakura() {
    let table = RegisteredMacros::from_profile(&profile(
        "[Macro]\nName[000]=整形\nFile[000]=fmt.mac\nReloadWhenExecute[000]=1\n\
         Name[049]=\nFile[049]=C:\\m\\last.mjs\nnMacroOnOpened=0\nnMacroOnTypeChanged=49\n\
         nMacroOnSave=50\nnMacroCancelTimer=3\n",
    ));
    assert_eq!(
        table.macros[0],
        MacroRec {
            name: "整形".into(),
            file: "fmt.mac".into(),
            reload: true
        }
    );
    // No name: the title is the file.
    assert_eq!(table.macros[49].title(), "C:\\m\\last.mjs");
    assert!(table.macros[49].is_enabled() && !table.macros[49].reload);
    assert_eq!(table.macros[0].title(), "整形");
    assert_eq!(table.on_opened, Some(0));
    assert_eq!(table.on_type_changed, Some(49));
    // 50 is outside the table, as -1 is.
    assert_eq!(table.on_save, None);
    assert_eq!(table.cancel_timer, 3);
}

#[test]
fn a_row_without_name_or_file_key_is_left_empty() {
    // `IOProfileData<MacroRec>` gives up at the first key that is missing.
    let table = RegisteredMacros::from_profile(&profile(
        "[Macro]\nFile[001]=only_file.mac\nName[002]=only name\nName[003]=n\nFile[003]=f.mac\n",
    ));
    assert_eq!(table.macros[1], MacroRec::default());
    assert_eq!(table.macros[2], MacroRec::default());
    assert!(table.macros[3].is_enabled());
}

#[test]
fn writing_keeps_the_rest_of_the_file_and_drops_empty_rows() {
    let mut file = profile(
        "[Common]\nx=1\n[Macro]\nName[005]=old\nFile[005]=old.mac\nReloadWhenExecute[005]=1\n",
    );
    let mut table = RegisteredMacros::from_profile(&file);
    table.macros[5] = MacroRec::default();
    table.macros[2] = MacroRec {
        name: "名前".into(),
        file: "a.mac".into(),
        reload: false,
    };
    table.on_save = Some(2);
    table.cancel_timer = 20;
    assert!(table.store(&mut file).unwrap());
    assert_eq!(file.get("Common", "x"), Some("1"));
    assert_eq!(file.get("Macro", "Name[002]"), Some("名前"));
    assert_eq!(file.get("Macro", "File[002]"), Some("a.mac"));
    assert_eq!(file.get("Macro", "ReloadWhenExecute[002]"), Some("0"));
    assert_eq!(file.get("Macro", "Name[005]"), None);
    assert_eq!(file.get("Macro", "File[005]"), None);
    assert_eq!(file.get("Macro", "ReloadWhenExecute[005]"), None);
    assert_eq!(file.get("Macro", "nMacroOnOpened"), Some("-1"));
    assert_eq!(file.get("Macro", "nMacroOnSave"), Some("2"));
    assert_eq!(file.get("Macro", "nMacroCancelTimer"), Some("20"));
    // Read back, it is the same.
    assert_eq!(RegisteredMacros::from_profile(&file), table);
}

#[test]
fn writing_what_the_file_has_changes_nothing() {
    let text = "[Macro]\nName[000]=a\nFile[000]=a.mac\nReloadWhenExecute[000]=+1\n\
                nMacroOnOpened=-1\nnMacroOnTypeChanged=-1\nnMacroOnSave=-1\nnMacroCancelTimer=10\n";
    let mut file = profile(text);
    let table = RegisteredMacros::from_profile(&file);
    assert!(!table.store(&mut file).unwrap());
    assert_eq!(file.to_bytes(), profile(text).to_bytes());
}

#[test]
fn a_function_code_is_a_macro_number() {
    assert_eq!(index_of_function(31600), Some(0));
    assert_eq!(index_of_function(31649), Some(49));
    assert_eq!(index_of_function(31650), None);
    assert_eq!(index_of_function(31599), None);
    assert_eq!(function_of_index(0), Some(31600));
    assert_eq!(function_of_index(49), Some(31649));
    assert_eq!(function_of_index(50), None);
    let table = RegisteredMacros::default();
    assert!(table.of_function(31601).is_some() && table.of_function(1).is_none());
}

#[test]
fn the_macro_folder_is_in_folders_and_ends_with_a_backslash() {
    assert_eq!(
        macro_folder(&profile("[Folders]\nszMACROFOLDER=C:\\macros\\\n")),
        Some("C:\\macros\\".into())
    );
    // `[Macro]` is not where Sakura keeps it.
    assert_eq!(
        macro_folder(&profile("[Macro]\nszMACROFOLDER=C:\\macros\\\n")),
        None
    );
    assert_eq!(with_last_separator("C:\\macros"), "C:\\macros\\");
    assert_eq!(with_last_separator("C:\\macros\\"), "C:\\macros\\");
    assert_eq!(with_last_separator("マクロ"), "マクロ\\");
    assert_eq!(with_last_separator(""), "");
    let mut file = profile("");
    assert!(store_macro_folder(&mut file, "m\\").unwrap());
    assert!(!store_macro_folder(&mut file, "m\\").unwrap());
    assert_eq!(file.get("Folders", "szMACROFOLDER"), Some("m\\"));
}

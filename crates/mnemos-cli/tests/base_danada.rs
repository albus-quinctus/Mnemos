//! La línea de órdenes ante una base de datos dañada (spec 022).

#[cfg(test)]
mod base_danada {
    use std::process::Command;

    use mnemos_core::Library;

    #[test]
    fn avisa_con_un_codigo_propio_y_no_toca_la_base() {
        let dir = tempfile::tempdir().unwrap();
        Library::create(dir.path()).unwrap();
        let db = dir.path().join(".mnemos").join("mnemos.db");
        let basura = vec![0x5a_u8; std::fs::metadata(&db).unwrap().len() as usize];
        std::fs::write(&db, &basura).unwrap();

        let out = Command::new(env!("CARGO_BIN_EXE_mnemos"))
            .args(["info", dir.path().to_str().unwrap()])
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(3));
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains("está dañada"), "{err}");
        assert!(err.contains("Tus fotos están bien"), "{err}");
        assert!(err.contains(".mnemos/copias/"), "{err}");
        assert_eq!(std::fs::read(&db).unwrap(), basura);
    }
}

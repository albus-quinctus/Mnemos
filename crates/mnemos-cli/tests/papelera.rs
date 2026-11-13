//! `mnemos papelera` y `mnemos papelera vaciar` (spec 015).

#[cfg(test)]
mod papelera {
    use std::io::Write;
    use std::path::Path;
    use std::process::{Command, Output, Stdio};

    use mnemos_core::Library;

    /// PNG mínimo (solo cabecera), distinto según `salt`.
    fn write_png(path: &Path, salt: &[u8]) {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&10u32.to_be_bytes());
        v.extend_from_slice(&10u32.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v.extend_from_slice(salt);
        std::fs::write(path, v).unwrap();
    }

    /// Biblioteca con «a.png» en la Papelera y «b.png» fuera.
    fn library() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        write_png(&dir.path().join("Originales/a.png"), b"a");
        write_png(&dir.path().join("Originales/b.png"), b"b");
        lib.index().unwrap();
        let a: i64 = lib
            .connection()
            .query_row(
                "SELECT id FROM photos WHERE rel_path = 'Originales/a.png'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        lib.send_to_trash(&[a]).unwrap();
        dir
    }

    fn mnemos(args: &[&str], stdin: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mnemos"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    fn text(out: &Output) -> String {
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[test]
    fn lista_la_papelera() {
        let dir = library();
        let lib = dir.path().to_str().unwrap();
        let out = mnemos(&["papelera", lib], "");
        assert!(out.status.success());
        let text = text(&out);
        assert!(text.contains("En la papelera: 1"), "{text}");
        assert!(text.contains("Originales/a.png  se elimina el "), "{text}");
        assert!(!text.contains("b.png"), "{text}");
    }

    #[test]
    fn vaciar_sin_decir_que_si_no_borra_nada() {
        let dir = library();
        let lib = dir.path().to_str().unwrap();
        let out = mnemos(&["papelera", "vaciar", lib], "no\n");
        assert!(out.status.success());
        assert!(text(&out).contains("No se ha borrado nada."));
        assert!(dir.path().join("Originales/a.png").exists());
    }

    #[test]
    fn vaciar_diciendo_que_si_borra_solo_la_papelera() {
        let dir = library();
        let lib = dir.path().to_str().unwrap();
        let out = mnemos(&["papelera", "vaciar", lib], "sí\n");
        assert!(out.status.success(), "{}", text(&out));
        assert!(text(&out).contains("Eliminadas: 1"));
        assert!(!dir.path().join("Originales/a.png").exists());
        assert!(dir.path().join("Originales/b.png").exists());
        let out = mnemos(&["papelera", lib], "");
        assert!(text(&out).contains("La papelera está vacía."));
    }

    #[test]
    fn vaciar_con_confirmar_no_pregunta() {
        let dir = library();
        let lib = dir.path().to_str().unwrap();
        let out = mnemos(&["papelera", "vaciar", lib, "--confirmar"], "");
        assert!(out.status.success());
        assert!(!dir.path().join("Originales/a.png").exists());
    }
}

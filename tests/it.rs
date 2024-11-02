use shx::{cmd, lex, shx, Result};

#[test]
fn echo() -> Result<()> {
    shx!(echo hello world)?;
    let mut out = String::new();
    shx!(echo hello world > {&mut out})?;
    assert_eq!(out, "hello world\n");
    out.clear();
    Ok(())
}

#[test]
fn multiple() {
    lex! {
        echo hello world;
        echo hello world;
    };
}

#[test]
fn literals() -> Result<()> {
    let mut out = String::new();
    shx!(echo "hello world" > {&mut out})?;
    assert_eq!(out, "hello world\n");
    out.clear();

    shx!(echo 1 2 3u8 > {&mut out})?;
    assert_eq!(out, "1 2 3u8\n");
    out.clear();

    shx!(echo 1.23 > {&mut out})?;
    assert_eq!(out, "1.23\n");
    out.clear();

    shx!(echo true > {&mut out})?;
    assert_eq!(out, "true\n");
    out.clear();
    Ok(())
}

#[test]
fn source_types() -> Result<()> {
    let mut out = String::new();
    let hello = "hello world";
    shx!(cat < {hello} > {&mut out})?;
    assert_eq!(&out, "hello world");
    out.clear();

    let hello = "hello world".to_string();
    shx!(cat < {hello} > {&mut out})?;
    assert_eq!(&out, "hello world");
    out.clear();

    let hello = b"hello world".as_slice();
    shx!(cat < {hello} > {&mut out})?;
    assert_eq!(&out, "hello world");
    out.clear();

    let hello = b"hello world".to_vec();
    shx!(cat < {hello} > {&mut out})?;
    assert_eq!(&out, "hello world");
    out.clear();
    Ok(())
}

#[test]
fn sink_types() -> Result<()> {
    let mut out = String::new();
    shx!(echo "hello world" > {&mut out})?;
    assert_eq!(out, "hello world\n");
    out.clear();

    let mut out = Vec::new();
    shx!(echo "hello world" > {&mut out})?;
    assert_eq!(out, b"hello world\n");
    out.clear();
    Ok(())
}

#[test]
fn error_conditions() {
    let bytes = "0012345678910";
    let mut out = String::new();
    let err = lex!(xxd "-r" "-p" < {bytes} > {&mut out})
        .next()
        .unwrap()
        .exec()
        .unwrap_err();
    assert!(matches!(err, shx::cmd::Error::NotUtf8));
}

#[test]
fn extra_args() -> Result<()> {
    let mut cmd = cmd!(ls);
    cmd.arg("-al");
    cmd.exec()
}


#[test]
fn variadic_expression() -> Result<()> {
    let path = ".";
    let option = Some("..");
    let list = &[".", ".."];
    shx!(ls {path} ...{option} ...{list})
}

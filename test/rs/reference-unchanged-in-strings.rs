Value::Object(HashMap::from([
    ("a".into(), Value::String("<https://x>".into())),
    ("b".into(), Value::String("<urn:x>".into())),
    ("c".into(), Value::Tagged("<https://x>".into(), Box::new(Value::Integer(1.into())))),
    ("d".into(), Value::String("<https://in-a-block-string>\n".into())),
]))

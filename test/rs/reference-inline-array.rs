Value::Object(HashMap::from([
    ("mirrors".into(), Value::Array(vec![
        Value::Reference("https://a.example/x".into()),
        Value::Reference("https://b.example/x".into()),
        Value::Bytes(vec![0xff]),
        Value::Bytes(vec![]),
    ])),
]))

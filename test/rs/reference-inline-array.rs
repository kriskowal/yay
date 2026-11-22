Value::Object(HashMap::from([
    ("mirrors".into(), Value::Array(vec![
        Value::reference("https://a.example/x").unwrap(),
        Value::reference("https://b.example/x").unwrap(),
        Value::Bytes(vec![0xff]),
        Value::Bytes(vec![]),
    ])),
]))

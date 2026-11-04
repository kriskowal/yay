Value::Object(HashMap::from([
    ("links".into(), Value::Tagged("mirrors".into(), Box::new(Value::Array(vec![
        Value::Reference("https://a.example/x".into()),
        Value::Reference("https://b.example/x".into()),
    ])))),
]))

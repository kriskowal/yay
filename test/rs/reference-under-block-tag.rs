Value::Object(HashMap::from([
    ("links".into(), Value::Tagged("mirrors".into(), Box::new(Value::Array(vec![
        Value::reference("https://a.example/x").unwrap(),
        Value::reference("https://b.example/x").unwrap(),
    ])))),
]))

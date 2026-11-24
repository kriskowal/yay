Value::Object(HashMap::from([
    ("q".into(), Value::Array(vec![
        Value::reference("https://x.example/a,b]c?e=(f)&g='h'").unwrap(),
        Value::Integer(1.into()),
    ])),
]))

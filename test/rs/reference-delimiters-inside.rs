Value::Object(HashMap::from([
    ("q".into(), Value::Array(vec![
        Value::Reference("https://x.example/a,b]c?e=(f)&g='h'".into()),
        Value::Integer(1.into()),
    ])),
]))

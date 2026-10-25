Value::Object(HashMap::from([
    ("list".into(), Value::Array(vec![
        Value::String("é".into()),
        Value::String("x".into()),
    ])),
    ("object".into(), Value::Object(HashMap::from([
        ("a".into(), Value::String("😀".into())),
        ("b".into(), Value::Integer(1.into())),
    ]))),
]))

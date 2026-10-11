Value::Object(HashMap::from([
    ("items".into(), Value::Array(vec![
        Value::Object(HashMap::from([
            ("a".into(), Value::Integer(1.into())),
            ("b".into(), Value::Integer(2.into())),
        ])),
        Value::Object(HashMap::from([
            ("body".into(), Value::String("First.\n\nSecond.\n".into())),
            ("title".into(), Value::String("a".into())),
        ])),
    ])),
    ("next".into(), Value::Integer(1.into())),
]))

Value::Object(HashMap::from([
    ("a".into(), Value::Object(HashMap::from([
        ("a".into(), Value::Integer(1.into())),
    ]))),
    ("b".into(), Value::Array(vec![
        Value::Object(HashMap::from([
            ("a".into(), Value::Integer(2.into())),
        ])),
        Value::Object(HashMap::from([
            ("a".into(), Value::Integer(3.into())),
        ])),
    ])),
    ("c".into(), Value::Object(HashMap::from([
        ("a".into(), Value::Object(HashMap::from([
            ("a".into(), Value::Integer(4.into())),
        ]))),
    ]))),
]))

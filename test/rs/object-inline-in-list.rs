Value::Array(vec![
    Value::Object(HashMap::from([
        ("a".into(), Value::Integer(1.into())),
    ])),
    Value::Object(HashMap::from([
        ("b".into(), Value::Array(vec![
            Value::Integer(1.into()),
            Value::Integer(2.into()),
        ])),
    ])),
])

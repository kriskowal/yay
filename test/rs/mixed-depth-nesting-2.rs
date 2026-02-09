Value::Array(vec![
    Value::Object(HashMap::from([
        (
            "a".into(),
            Value::Object(HashMap::from([(
                "b".into(),
                Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
            )])),
        ),
        ("c".into(), Value::Integer(3.into())),
    ])),
    Value::Array(vec![
        Value::String("d".into()),
        Value::Object(HashMap::from([(
            "e".into(),
            Value::Object(HashMap::from([("f".into(), Value::Integer(4.into()))])),
        )])),
    ]),
])

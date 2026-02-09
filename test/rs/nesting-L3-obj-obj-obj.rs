Value::Object(HashMap::from([(
    "a".into(),
    Value::Object(HashMap::from([(
        "b".into(),
        Value::Object(HashMap::from([
            ("c".into(), Value::Integer(42.into())),
            ("d".into(), Value::String("hello".into())),
        ])),
    )])),
)]))

Value::Object(HashMap::from([(
    "top".into(),
    Value::Object(HashMap::from([
        (
            "list".into(),
            Value::Array(vec![
                Value::Object(HashMap::from([
                    ("bytes".into(), Value::Bytes(vec![0xb0, 0xb5])),
                    ("tag".into(), Value::String("x".into())),
                ])),
                Value::Object(HashMap::from([(
                    "more".into(),
                    Value::Object(HashMap::from([(
                        "inner".into(),
                        Value::Bytes(vec![0x0f, 0x0f]),
                    )])),
                )])),
            ]),
        ),
        ("solo".into(), Value::Integer(1.into())),
    ])),
)]))

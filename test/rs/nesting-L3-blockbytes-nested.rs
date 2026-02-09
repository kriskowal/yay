Value::Object(HashMap::from([(
    "outer".into(),
    Value::Object(HashMap::from([(
        "inner".into(),
        Value::Object(HashMap::from([(
            "data".into(),
            Value::Bytes(vec![0xca, 0xfe]),
        )])),
    )])),
)]))

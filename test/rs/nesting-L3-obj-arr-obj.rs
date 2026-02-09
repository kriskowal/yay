Value::Object(HashMap::from([(
    "items".into(),
    Value::Array(vec![
        Value::Object(HashMap::from([
            ("name".into(), Value::String("hello".into())),
            ("value".into(), Value::Integer(42.into())),
        ])),
        Value::Object(HashMap::from([
            ("name".into(), Value::String("hello".into())),
            ("value".into(), Value::Integer(42.into())),
        ])),
    ]),
)]))

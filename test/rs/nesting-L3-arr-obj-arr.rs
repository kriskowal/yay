Value::Array(vec![
    Value::Object(HashMap::from([(
        "data".into(),
        Value::Array(vec![Value::Integer(42.into()), Value::Integer(42.into())]),
    )])),
    Value::Object(HashMap::from([(
        "data".into(),
        Value::Array(vec![
            Value::String("hello".into()),
            Value::String("hello".into()),
        ]),
    )])),
])

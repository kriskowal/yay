Value::Object(HashMap::from([(
    "items".into(),
    Value::Array(vec![
        Value::Array(vec![Value::Integer(42.into()), Value::Integer(42.into())]),
        Value::Array(vec![
            Value::String("hello".into()),
            Value::String("hello".into()),
        ]),
    ]),
)]))

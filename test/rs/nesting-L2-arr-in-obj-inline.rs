Value::Object(HashMap::from([
    (
        "a".into(),
        Value::Array(vec![Value::Integer(42.into()), Value::Integer(42.into())]),
    ),
    (
        "b".into(),
        Value::Array(vec![
            Value::String("hello".into()),
            Value::String("hello".into()),
        ]),
    ),
]))

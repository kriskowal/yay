Value::Object(HashMap::from([(
    "root".into(),
    Value::Object(HashMap::from([
        (
            "a".into(),
            Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
        ),
        (
            "b".into(),
            Value::Object(HashMap::from([
                ("c".into(), Value::Array(vec![Value::Integer(3.into())])),
                ("d".into(), Value::Integer(4.into())),
            ])),
        ),
    ])),
)]))

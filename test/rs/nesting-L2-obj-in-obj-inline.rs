Value::Object(HashMap::from([
    (
        "a".into(),
        Value::Object(HashMap::from([("x".into(), Value::Integer(42.into()))])),
    ),
    (
        "b".into(),
        Value::Object(HashMap::from([("y".into(), Value::String("hello".into()))])),
    ),
]))

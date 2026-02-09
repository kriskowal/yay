Value::Object(HashMap::from([
    ("air".into(), Value::Array(vec![
        Value::String("canned".into()),
        Value::String("Perri-Air".into()),
    ])),
    ("luggage".into(), Value::Object(HashMap::from([
        ("combination".into(), Value::Integer(12345.into())),
    ]))),
]))

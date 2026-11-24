Value::Tagged("match:splitRecord".into(), Box::new(Value::Array(vec![
    Value::Object(HashMap::new()),
    Value::Tagged("a: b".into(), Box::new(Value::String("x".into()))),
])))

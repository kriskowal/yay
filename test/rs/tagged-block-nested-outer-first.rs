Value::Array(vec![
    Value::Tagged("outer".into(), Box::new(Value::Tagged("inner".into(), Box::new(Value::Array(vec![Value::Integer(1.into())]))))),
])

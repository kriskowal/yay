Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("t".into(), Box::new(Value::String("x".into())))),
    ("b".into(), Value::Tagged("s".into(), Box::new(Value::Integer(1.into())))),
    ("c".into(), Value::Tagged("a)  # b".into(), Box::new(Value::Integer(2.into())))),
]))

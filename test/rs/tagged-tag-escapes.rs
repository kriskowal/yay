Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("a)b".into(), Box::new(Value::Integer(1.into())))),
    ("b".into(), Value::Tagged("back\\slash".into(), Box::new(Value::Integer(2.into())))),
    ("c".into(), Value::Tagged("tab\there".into(), Box::new(Value::Integer(3.into())))),
    ("d".into(), Value::Tagged("😀".into(), Box::new(Value::Integer(4.into())))),
    ("e".into(), Value::Tagged("a/b".into(), Box::new(Value::Integer(5.into())))),
    ("f".into(), Value::Tagged("\n".into(), Box::new(Value::Integer(6.into())))),
]))

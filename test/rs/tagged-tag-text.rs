Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("com.example.sku".into(), Box::new(Value::Integer(1.into())))),
    ("b".into(), Value::Tagged("match:any".into(), Box::new(Value::Integer(2.into())))),
    ("c".into(), Value::Tagged("image/png".into(), Box::new(Value::Integer(3.into())))),
    ("d".into(), Value::Tagged("my tag".into(), Box::new(Value::Integer(4.into())))),
    ("e".into(), Value::Tagged("a#b".into(), Box::new(Value::Integer(5.into())))),
    ("f".into(), Value::Tagged("a  # b".into(), Box::new(Value::Integer(6.into())))),
    ("g".into(), Value::Tagged("it's".into(), Box::new(Value::Integer(7.into())))),
    ("h".into(), Value::Tagged("a(b".into(), Box::new(Value::Integer(8.into())))),
    ("i".into(), Value::Tagged("\"q\"".into(), Box::new(Value::Integer(9.into())))),
    ("j".into(), Value::Tagged("a, b] }".into(), Box::new(Value::Integer(10.into())))),
]))

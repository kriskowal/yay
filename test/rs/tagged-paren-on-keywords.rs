Value::Object(HashMap::from([
    ("a".into(), Value::Tagged("t".into(), Box::new(Value::Bool(true)))),
    ("b".into(), Value::Tagged("null".into(), Box::new(Value::Null))),
    ("c".into(), Value::Tagged("ms".into(), Box::new(Value::Float(f64::NAN)))),
    ("d".into(), Value::Tagged("x".into(), Box::new(Value::Float(f64::NEG_INFINITY)))),
]))

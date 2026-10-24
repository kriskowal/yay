Value::Object(HashMap::from([
    ("meta".into(), Value::Object(HashMap::from([
        ("A quoted key".into(), Value::String("first in its object".into())),
        ("b".into(), Value::Integer(2.into())),
    ]))),
]))

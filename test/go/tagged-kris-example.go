map[string]any{
	"color": yay.Tagged{Tag: "rgba", Payload: []byte{0xff, 0x88, 0x00, 0xff}},
	"count": yay.Tagged{Tag: "s", Payload: big.NewInt(10)},
	"odd": yay.Tagged{Tag: "a)b", Payload: big.NewInt(3)},
	"sku": yay.Tagged{Tag: "com.example/sku", Payload: "X-1"},
	"when": yay.Tagged{Tag: "utc", Payload: yay.Tagged{Tag: "datetime", Payload: "2026-11-03T05:00:00Z"}},
}

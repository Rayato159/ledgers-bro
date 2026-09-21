# katgpt-rs กับ Ledgers Bro

ตรวจวันที่ 20 กันยายน 2026 จาก checkout `D:/SideProjects/katgpt-rs` commit `7614f24bc124690394b10b038bbb5c653216ded4` (working tree สะอาดขณะตรวจ) เทียบเอกสาร upstream ด้วย เป็นการอ่านโค้ด/เอกสาร **ยังไม่ได้ build benchmark โหลดโมเดล หรือทดสอบ Android ของ repo นี้** และยังไม่ได้เพิ่ม dependency ใน Ledgers Bro

## ข้อสรุป

ใช้เป็นแหล่ง inference primitives หรือทดลอง adapter ได้ แต่ยังไม่เลือกเป็น runtime หลักของแอปจดเงิน ตัวที่ต้องเลือกแยกกันคือ **runtime + model weights + tokenizer/chat template + output constraints** การใส่ system prompt อย่างเดียวไม่ทำให้โครงสร้างเหล่านี้ครบหรือทำให้ผลบัญชี deterministic

## พบอะไรในโค้ดจริง

| ส่วน | สิ่งที่ตรวจพบ | ความหมายกับแอปเรา |
|---|---|---|
| `BOUNDARY.md` | Public modelless inference primitives อยู่ upstream ของกลุ่ม riir; runtime/SDK หลายส่วนเป็น consumer ภายนอก | อย่าเหมาว่าทุกความสามารถของ riir อยู่ใน public crate นี้ |
| `katgpt-backend/src/lib.rs` | มี forward-pass `InferenceBackend` และ `CpuBackend` เรียก `katgpt_forward::forward` จริง | มีงาน inference จริง ไม่ใช่ทุกส่วนเป็น mock |
| `katgpt-core/src/prompt_backend.rs` | trait `generate(prompt) -> Option<String>` และ `CannedResponseBackend` คืนข้อความที่ใส่ไว้ | ส่วน prompt→ข้อความนี้ยังต้องมี implementation ที่โหลดโมเดล/ทำ generation; mock ไม่ใช่หลักฐานว่ารัน LLM |
| `katgpt-backend/Cargo.toml` + `lib.rs` | GPU/ANE ที่ตรวจเป็น optional macOS Metal/CoreML และ gated `target_os = macos` | ไม่ใช่เส้นทาง Android GPU/NPU ที่เปิดใช้แทนได้ทันที; ARM/NEON kernels ไม่เท่ากับ APK/mobile lifecycle ที่ผ่านทดสอบ |
| `katgpt-core/src/traits/mod.rs` | `ConstraintPruner` คัด candidate tokens ด้วยกฎ; มี speculative decoding substrate | อาจใช้บังคับโครงสร้าง/ตัดทางเลือกผิดได้ แต่ต้องเขียนและทดสอบ JSON/tokenizer constraints สำหรับ schema เราเอง |
| `katgpt-validator/src/lib.rs` | bracket parser + `syn::parse_str::<syn::Stmt>` ตรวจไวยากรณ์ Rust | ไม่ใช่ตัวตรวจ JSON schema, เลขเงิน, ID บัญชี หรือกฎหมายภาษี |
| `katgpt-tokenizer` | มี BPE และตัวเลือก tokenization อื่น | ต้องพิสูจน์ tokenizer ตรงกับ weights และภาษาไทย ไม่มีเหตุให้อนุมานความแม่นไทยจากชื่อ BPE |
| `src/kimi_k3/loader.rs` | มี loader เฉพาะโมเดลจาก safetensors; คอมเมนต์ระบุ materialize weights เป็น `Vec<f32>` ราว 1.5 GB | ไม่ใช่หลักฐานว่าเป็นโมเดลมือถือที่เบาสุด; ตัวเลขนี้เป็นข้อมูลในโค้ด ไม่ใช่ RAM ที่วัดจากเครื่องเรา |
| `katgpt-types/src/enums.rs` | มี architecture variants หลายแบบ/feature gates; บางส่วนอ้าง forward dispatch ใน riir-engine | enum ว่า Gemma/Llama/Qwen หรือ GGUF ไม่พิสูจน์ว่า public repo โหลด instruct model ใดก็ได้ครบทุกชนิด |
| `.github/workflows/test.yml`, workflows และ source ที่ค้น | เจอ host/wasm gates; ไม่พบ Android target/ชุดทดสอบการสกัดรายการเงินภาษาไทยในพื้นที่ที่ตรวจ | ยังต้องพิสูจน์ x86_64 emulator และ ARM64 จริง ไม่กล่าวว่า Android ใช้ไม่ได้โดยสิ้นเชิง |

README มีผล benchmark ของ kernels/arenas หลายชนิด; ตัวเลขนั้นไม่ใช่ผลวัด Thai financial extraction, เวลาเปิด model, peak RAM หรือการกินแบตของแอปนี้ คำว่า GOAT/proved ต้องอ่านสมมติฐานและ fixture ของแต่ละ gate ไม่ใช่ใบรับรองความถูกต้องบัญชี

## เส้นทางใช้กับเรา

```text
ข้อความผู้ใช้
  → grammar ที่รองรับแน่นอน
  → ถ้าไม่ตรงรูปแบบ: local model adapter เสนอ JSON
  → validate_proposal (schema + ข้อความอ้างอิงจากต้นฉบับ)
  → resolve IDs / validate เงิน วันที่ และกฎรายการด้วย Rust
  → แสดง draft หรือ choices ให้ผู้ใช้เลือก
  → Preview → ยืนยัน → Commit ผ่าน application/repository
```

`domain` และ `application` ไม่ควร import katgpt; หากทดลอง ให้ adapter อยู่ infrastructure และมี application-owned port ใช้เพียง crate/features ที่ต้องการพร้อม pin revision ไม่เปิด default/full ทั้งต้นไม้เพราะจำนวน feature เยอะ ไม่ส่งข้อมูลบัญชีจริงออก cloud ไม่ให้ model แตะ SQLite และไม่ให้จัด VAT/WHT หรือคำนวณเดบิตเครดิตเอง

อย่าสับสน **deterministic decoding** กับ **deterministic accounting**: greedy/seed/pinned runtime ช่วยทวนผล แต่ kernel/quantization/hardware ต่างกันยังต้องตรวจซ้ำ กฎเงินและผล commit ต้อง deterministic จากข้อมูลที่ยืนยันแล้วเสมอ แม้ข้อความที่ model เสนอจะต่างกัน

## ด่านก่อนเลือก runtime

1. ระบุ checkpoint instruct ที่รองรับไทยและมี license ใช้เชิงพาณิชย์ได้; ตรวจ loader, tensors, tokenizer, chat template และ quantization จริง
2. ใช้ชุดทดสอบอิสระ: ตัวเลขไทย/อารบิก, ทศนิยม/คอมมา, หลายบัญชีชื่อใกล้กัน, หลายจำนวนในข้อความ, ไม่ระบุบัญชี/วันที่, transfer/หนี้, VAT+WHT, OCR กำกวม, คำสั่งแฝงในชื่อบัญชี/โน้ต และ unsupported intents
3. วัด exact amount match, intent/account/category correctness, missing-field/unsupported handling, JSON validity และอัตราเสนอข้อมูลเกินต้นฉบับ; invalid ต้องไม่เขียนสมุดบัญชี
4. วัดไฟล์รวม/tokenizer, cold start, peak RAM, p50/p95 latency, cancel/timeout, background/resume, process death และ battery/thermal บน ARM64 จริง Emulator ใช้ตรวจฟังก์ชันและ lifecycle เบื้องต้น ไม่ใช้ฟันธงความเร็วมือถือ
5. เปรียบเทียบกับ baseline runtime ที่โหลด checkpoint เดียวกันได้จริง ปรับเพียงตัวแปรที่วัด และเก็บผลซ้ำหลายรอบ การเร่ง speculative decoding ต้องคิด memory ของ draft model เพิ่มด้วย

โค้ด repo เป็น MIT ตาม `LICENSE` (ต้องเก็บ notice); license ของ model weights/data เป็นอีกเรื่องหนึ่ง ยังไม่มีหลักฐานพอให้เรียก backend ใดว่า “ดีที่สุดและเบาที่สุด” สำหรับผู้ใช้แอปนี้

แหล่งต้นทาง: [repo](https://github.com/katopz/katgpt-rs), [backend](https://github.com/katopz/katgpt-rs/blob/main/crates/katgpt-backend/src/lib.rs), [prompt backend](https://github.com/katopz/katgpt-rs/blob/main/crates/katgpt-core/src/prompt_backend.rs), [validator](https://github.com/katopz/katgpt-rs/blob/main/crates/katgpt-validator/src/lib.rs), [license](https://github.com/katopz/katgpt-rs/blob/main/LICENSE). การอ้างพฤติกรรมเฉพาะในตารางยึด commit ที่ระบุด้านบน เพราะ upstream main อาจเปลี่ยนหลังตรวจ

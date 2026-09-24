# AI บันทึกด่วนในเครื่อง

อัปเดต 24 กันยายน 2026: เพิ่ม grammar ประโยคไทยและหลายรายการใน Rust (ไม่เรียกโมเดล) พร้อมแจ้งข้อมูลที่ขาดและยืนยันแบบ atomic batch ดู [ตัวอย่างและผลทดสอบ](verification-prompt-flow-tax-th.md) ขอบเขต LLM ยังเป็นการตีความทางเลือกของรายการเดียว; ถ้าแบ่งหลายรายการไม่สำเร็จจะไม่ส่งให้โมเดลเลือกเฉพาะรายการแรก ข้อความอิสระนอก grammar ยังต้องดาวน์โหลดโมเดลก่อน

สถานะ 21 กันยายน 2026: ต่อ LLM จริงแล้ว เป็น development build สำหรับทดลอง ไม่ใช่คำรับรองความแม่นยำทุกข้อความหรือความพร้อมขาย

## ลองใช้

1. Windows: `./scripts/run-desktop.ps1` หรือ Android: build/install ตาม `docs/android-emulator-th.md`
2. เพิ่มบัญชี แล้วเข้า **บันทึกด่วน → AI ในเครื่อง → ดาวน์โหลด AI ในเครื่อง** ใช้อินเทอร์เน็ตครั้งแรกประมาณ 397 MB ต้องมีพื้นที่ว่างสำหรับไฟล์ชั่วคราวและโมเดล
3. พิมพ์ เช่น `เมื่อวานซื้อกาแฟ 80 บาท จ่ายเงินสด` หรือ `จายค่าอาหาร 120 บาทจากเงินสด`
4. อ่านข้อความต้นฉบับเทียบตัวเลือก กด **เลือกและตรวจรายละเอียด** เลือกบัญชี/หมวดที่ขาด แล้ว **ตรวจรายการก่อนบันทึก → ยืนยันบันทึก**

หลังดาวน์โหลด ข้อความประมวลผลบน CPU ของเครื่อง ไม่มี cloud inference และไม่ต้องเปิด server แยก Windows ใช้ `<data-directory>/models`; Android เก็บใน private no-backup directory ไม่รวมโมเดลลง APK

สำหรับโมเดลที่ดาวน์โหลดไว้ใน workspace แล้ว:

```powershell
./scripts/run-desktop.ps1 -ModelDirectory "$PWD/.tools/llm"
```

ถ้าไม่มีโมเดล ยังใช้ `กาแฟ 80` หรือ `จ่าย 80 จาก เงินสด หมวด อาหาร` และแบบฟอร์มได้ การพิมพ์ตามรูปแบบนี้ใช้ Rust parser โดยไม่เรียก LLM

## การทำงานและขอบเขต

`ข้อความ → parser ถ้าตรงรูปแบบ / LLM ถ้าเป็นประโยคอิสระ → ตรวจผลใน application → ตัวเลือก → แบบฟอร์ม → ตรวจบัญชี → ผู้ใช้ยืนยัน → SQLite`

- LLM ตีความเจตนาและเสนอข้อความที่คัดจากต้นฉบับ ไม่มี repository, SQL, tool call หรือสิทธิ์บันทึก
- ใช้ Qwen3 0.6B Q4_K_M, llama-cpp-2 **0.1.156**, CPU 4 threads, context 2,048 tokens, output ไม่เกิน 512 tokens, greedy sampler และ grammar บังคับโครงสร้าง JSON ไม่อ้างว่าให้ผลเหมือนกันทุกอุปกรณ์
- จำกัดค่าจำนวนเงิน สกุลเงิน และวันที่ที่ sampler เลือกได้จากข้อความจริง ตรวจ schema, ขนาด, ชนิดรายการ และ source grounding ซ้ำใน Rust
- ถ้าโมเดลแก้ชื่อบัญชีหรือแต่งหมวด/รายละเอียดที่ไม่มีในต้นฉบับ จะเว้นช่องนั้นและบอกผู้ใช้ให้เลือกเอง ไม่ผูกชื่อที่เดากับ AccountId แม้บังเอิญตรงกับบัญชีจริง ตัวตรวจ strict contract ยังปฏิเสธข้อความแต่งตามเดิม; review adapter ลดเฉพาะช่องข้อความเหล่านี้เป็น null
- จำนวนเงิน/สกุลเงิน/วันที่ผิด contract, JSON ผิด หรือมี authority fields เกิน schema: ปฏิเสธ ไม่ซ่อมยอดจากโมเดล
- เงินคำนวณด้วย integer satang ใน domain บัญชี/วันที่ตรวจอีกครั้งตอน preview และ commit ยืนยันแล้วบันทึกแบบ atomic + idempotent
- ทุกผล LLM เป็นตัวเลือก แม้มีความหมายเดียว รองรับ 1–3 การตีความของรายการเดียว ไม่ใช่หลายรายการที่บันทึกพร้อมกัน
- มี conservative guards สำหรับคำปฏิเสธ/สมมติ/ลบ/แก้/หลายรายการ และสกุลเงินต่างประเทศ ไม่ใช่ตัวพิสูจน์ความหมายภาษาไทยทั้งหมด ข้อความบางแบบที่ถูกต้องก็อาจต้องใช้แบบฟอร์ม
- ภาษี, VAT + หัก ณ ที่จ่าย, ยอดก่อน/หลังหัก, การกู้ยืม, ซื้อขายสินทรัพย์ และรายการหลายชุด ยังอยู่นอกสัญญา LLM รุ่นนี้ ไม่ให้โมเดลคำนวณแทน tax engine
- วันที่ตั้งต้นคือวันนี้ แสดงให้ตรวจ วันที่ที่แปลงไม่ได้ต้องเลือกเอง; ไม่รับวันที่อนาคต ตัวเลขเขียนเป็นคำและรูปแบบตัวเลขที่ parser ไม่รองรับต้องกรอกเอง
- ช่องนี้ตีความทีละข้อความ ยังไม่มีความจำสนทนาข้ามข้อความ, vector database, alias learning หรือ fuzzy account ranking

คำสั่งสั้นตามรูปแบบทำงานทันที ส่วน LLM ใช้เวลาหลายวินาที หน้าจอแสดงขั้นตรวจไฟล์/เปิด AI/อ่านข้อความ/จัดรายละเอียดและเวลาจริง กด **ยกเลิกการอ่านรายการ** หรือออกจากหน้าได้ ไม่แก้ข้อมูลเงิน UI คืนฟอร์มเมื่อยกเลิกโดยไม่ต้องรอ native call จบ และจำกัดเวลารอรวม 90 วินาทีตั้งแต่เริ่มคำขอ งานโมเดลอยู่คนละ thread กับฐานข้อมูล คิวมีขอบเขต ตรวจยกเลิกระหว่าง chunks/tokens; native call ที่กำลังทำงานจะหยุดเมื่อถึงจุดตรวจถัดไป ผลที่มาหลังยกเลิกไม่ถูกนำมาใช้ โมเดลคืนหน่วยความจำเมื่อว่าง 60 วินาที

ขณะดาวน์โหลดมีปุ่มยกเลิกและตรวจ SHA-256 ก่อนย้ายไฟล์ชั่วคราวเป็นโมเดลพร้อมใช้ การยกเลิกอาจรอ network read ได้ถึง 20 วินาที ข้อความและยอดไม่ถูกส่งไปกับคำขอดาวน์โหลด

## โมเดลและที่มา

- ต้นทาง: [Qwen3-0.6B](https://huggingface.co/Qwen/Qwen3-0.6B)
- ไฟล์ GGUF: [Unsloth/Qwen3-0.6B-GGUF](https://huggingface.co/unsloth/Qwen3-0.6B-GGUF)
- Revision: `50968a4468ef4233ed78cd7c3de230dd1d61a56b`
- ไฟล์: `Qwen3-0.6B-Q4_K_M.gguf`, **396,705,472 bytes**
- SHA-256: `ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a`
- [Qwen chat template](https://huggingface.co/Qwen/Qwen3-0.6B/blob/main/tokenizer_config.json) ใช้ non-thinking assistant prefix; ไม่รองรับการแทนไฟล์ด้วยโมเดลอื่นโดยใช้ template เดิม
- [Rust wrapper](https://github.com/utilityai/llama-cpp-rs), [llama.cpp Android](https://github.com/ggml-org/llama.cpp/blob/master/docs/android.md)
- รวม Apache-2.0/MIT notices ใน `licenses/local-ai` และแสดงในหน้าบันทึกด่วน โมเดลไม่มีการ fine-tune โดยโปรเจกต์นี้

โมเดลนี้เป็น baseline ขนาดเล็กที่ทดสอบจริง ไม่ได้อ้างว่าแม่นที่สุดหรือเบาที่สุด ต้องมีชุดประเมินภาษาไทยกว้างขึ้นและมือถือจริงหลายระดับ RAM ก่อนขาย ดูผลตรวจที่ `docs/verification-local-llm.md`

## โครงสร้างสำหรับพัฒนา

`application/local_model.rs` เป็น port, `application/model_resolution.rs` กำหนดการแปลงผลเป็น draft, `infrastructure/local_model.rs` รัน native inference, `infrastructure/model_worker.rs` ดูแลโมเดลและคิว, UI ติดต่อผ่าน `UiGateway` เท่านั้น

`prompts/quick-entry-system.txt` เป็น policy contract ฉบับเต็ม ส่วน `prompts/quick-entry-runtime.txt` คือ prompt กระชับที่ runtime ใช้จริง ควบคู่ `prompts/quick-entry.gbnf` + source-specific rules

ตรวจตัวอย่างด้วยโมเดลจริง (ใช้ข้อความสังเคราะห์และพิมพ์ผลออก console; อย่าใส่ข้อมูลส่วนตัวใน fixture):

```powershell
. ./scripts/with-native-tools.ps1
Initialize-LedgerNativeTools
cargo run -p ledger-infrastructure --example local_model_probe --locked -- .tools/llm
```

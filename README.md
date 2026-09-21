# Ledgers Bro

แอปสมุดบัญชีในเครื่องด้วย Rust + Dioxus มี Windows และรุ่นทดลอง Android; iOS ยังไม่พอร์ต

สถานะ 21 กันยายน 2026: **มีแอป Desktop และ APK รุ่นทดลอง Android x86_64/ARM64 เป็น development build ยังไม่ใช่รุ่นพร้อมขายบน Store** ตรวจเปิดแอปบน Emulator แล้ว ส่วนมือถือ ARM64 ยังไม่ได้ทดสอบบนเครื่องจริง ไม่มี backend สมุดบัญชี ไม่มี analytics และไม่ส่งข้อความหรือยอดเงินไปบริการ AI

## ลองใช้บน Windows

ต้องมี Rust MSVC toolchain, Visual Studio C++ Build Tools, CMake, libclang (LLVM หรือ Android NDK) และ WebView2 ตาม [Dioxus setup](https://dioxuslabs.com/learn/0.7/getting_started/) ใช้ Dioxus 0.7.2 และตรึง dependency graph ใน `Cargo.lock` สคริปต์จะค้นหา libclang ให้โดยไม่เปลี่ยน environment ถาวร

```powershell
# สมุดบัญชีทดลองแยกจากข้อมูลส่วนตัว
./scripts/run-desktop.ps1

# โฟลเดอร์ข้อมูลส่วนตัวของแอปใน Windows
./scripts/run-desktop.ps1 -Personal
```

หรือเรียก `./scripts/run-desktop.ps1` เพื่อเปิดสมุดบัญชีทดลอง ภาพทานูกิแบบ doodles, CSS และฟอนต์ฝังใน executable ไม่ต้องเชื่อมเครือข่ายโหลดภาพหรือฟอนต์

**อ่านใบเสร็จ:** เข้า **บันทึกด่วน → เลือกรูปใบเสร็จ → ตรวจยอด/วันที่ → เลือกบัญชีและหมวด → ยืนยัน** รองรับ JPG/PNG/HEIC/HEIF ไม่เกิน 32 MB บน Windows และ Android 9 ขึ้นไป อ่านไทย/อังกฤษในเครื่อง Windows ติดตั้งครั้งแรกด้วย `./scripts/setup-ocr.ps1`; APK รวมตัวอ่านและโมเดลภาษาไว้แล้ว ดู [Receipt OCR](docs/receipt-ocr.md) ภาพใช้ชั่วคราวสำหรับตรวจรายการ ยังไม่แนบเก็บในฐานข้อมูล

เริ่มจาก **เพิ่มบัญชีแรก → ใส่ยอดเริ่มต้น → บันทึกด่วน → พิมพ์ `กาแฟ 80` → เลือกบัญชี/หมวด → ตรวจรายการ → ยืนยัน** แล้วปิดเปิดด้วย data directory เดิมเพื่อตรวจยอด

**กรอกเองโดยไม่ใช้ AI:** กด **กรอกเอง** ด้านบน หรือปุ่มกรอกเองในหน้าบันทึกด่วน → เลือกรายจ่าย/รายรับ/โอนเงิน → กรอกยอดและเลือกบัญชี/หมวด → ตรวจรายการ → ยืนยัน วันที่เริ่มต้นเป็นวันนี้ รายละเอียดยังแก้ได้ก่อนยืนยัน ถ้า AI ใช้เวลานาน กด **ยกเลิกแล้วกรอกเอง** เพื่อเริ่มแบบฟอร์มเปล่าได้

ข้อมูลอยู่ที่ `<data-directory>/ledger.sqlite3` โหมดปกติใช้ `ProjectDirs::data_local_dir()` ของ Windows ปัจจุบันฐานข้อมูลยังไม่เข้ารหัส ไม่มี PIN/biometric และยังไม่มี encrypted backup/restore ควรใช้ข้อมูลทดลองระหว่างพัฒนา **CSV เป็นรายงานส่งออก ไม่ใช่ไฟล์สำรองที่กู้คืนสมุดบัญชีได้ครบ**

## ทำงานแล้ว

- บัญชี 5 ประเภท สูงสุด 100 บัญชี รวมบัญชีที่อาจเก็บเข้าคลังในอนาคต ชื่อบัญชีไม่ซ้ำ
- ลบบัญชีพร้อมรายการที่เกี่ยวข้องผ่านหน้าต่างยืนยัน แสดงจำนวนรายการและยอดก่อน–หลังของบัญชีคู่โอน ลบรายการโอนทั้งสองฝั่งพร้อมประวัติการยกเลิก และย้อนคืนทั้งหมดหากลบไม่สำเร็จ
- รายรับ รายจ่าย โอนเงิน ยอดเริ่มต้น และยกเลิกรายการด้วย reversal ที่เก็บประวัติ
- สมุดบัญชีสองฝั่ง เงินบาทเป็นจำนวนเต็มสตางค์ ไม่มี float ในกฎบัญชี
- SQLite พร้อม migration, foreign keys, atomic transaction, idempotency และตรวจ postings เมื่ออ่าน
- สินทรัพย์ หนี้สิน สินทรัพย์สุทธิ รายรับ/รายจ่ายเดือนนี้ กราฟหมวด และประวัติ
- คำสั่ง deterministic พร้อมตัวอย่าง แบบฟอร์ม ตัวเลือกเมื่อกำกวม และยืนยันก่อนบันทึก
- หน้ากรอกเองแยกจากแชต ไม่ต้องติดตั้งหรือเรียก AI พร้อมยกเลิกการอ่านด้วย AI แล้วกลับมาใช้แบบฟอร์มได้
- ปุ่มพูดรายการบน Android ผ่าน on-device speech recognizer ภาษาไทย เติมข้อความให้แก้ก่อนอ่าน/ยืนยัน ต้องมีบริการและโมเดลออฟไลน์ที่รองรับ; Windows ยังไม่มีตัวถอดเสียง ดู [วิธีใช้และข้อจำกัดเสียง](docs/voice-entry-th.md)
- CSV บัญชีคู่: สมุดรายวันทั่วไป บัญชีแยกประเภท และงบทดลอง ผ่านหน้าต่างเลือกไฟล์ Windows ภาษาไทยเป็นค่าเริ่มต้น เลือก English ได้ พร้อม UTF-8 BOM และป้องกัน spreadsheet formula injection
- อ่านใบเสร็จผ่าน OCR ไทย/อังกฤษในเครื่องบน Windows/Android รวม HEIC/HEIF พร้อมแก้แนวภาพก่อนแสดงและอ่าน มีภาพเทียบ ข้อความต้นทาง ตัวเลือกยอด/วันที่ และยกเลิกการอ่าน
- รายการจากใบเสร็จแก้ได้ทีละบรรทัด บันทึกเป็น bullet ในรายละเอียด และบังคับตรวจผลรวมตรงยอดสุทธิก่อนยืนยัน แยกส่วนลด ภาษี/ค่าบริการบวกเพิ่ม และยอดที่รวมแล้ว
- UI โทนม่วงลาเวนเดอร์ ชมพู–พีช การ์ดขาวมุมมนและปุ่มไล่สี เมนูบันทึกด่วนกลางแถบล่าง ทานูกิลายเส้น doodles และไอคอนหมวด/ประเภทบัญชีสีพาสเทล
- Local LLM ผ่าน llama.cpp: ข้อความอิสระ → JSON ที่จำกัดรูปแบบและอ้างข้อความต้นทาง → ตัวเลือก → แบบฟอร์ม → ยืนยัน; คำสั่งตรงรูปแบบยังใช้ Rust parser ได้โดยไม่มีโมเดล

บัตรเครดิตแสดงยอดหนี้ ไม่ใช่วงเงิน รูดบัตรเป็นรายจ่ายและเพิ่มหนี้ โอนธนาคารจ่ายบัตรลดหนี้โดยไม่เพิ่มรายจ่ายซ้ำ Crypto/พอร์ตหุ้นขณะนี้จดมูลค่าเงินบาทด้วยมือ ไม่มี holdings, FX, ราคาสด หรือ valuation adjustments

## โครงสร้างและตรวจงาน

```text
apps/desktop          composition root + native file dialog
       ├── crates/ui                 Dioxus presentation
       └── crates/infrastructure     SQLite + clock + IDs + worker
                     │
             crates/application     use cases + ports + parser + projections
                     │
               crates/domain        entities + value objects + rules
```

อ่าน [สถาปัตยกรรม](docs/architecture.md) และ [Code conventions](CONTRIBUTING.md)

Android: build ด้วย `./scripts/build-android.ps1` แล้วติดตั้ง/เปิดบน emulator ด้วย `./scripts/run-android.ps1 -Device emulator-5554` ชื่อแอป **Ledgers Bro Test** มีข้อมูลแยกจาก Windows, Export ผ่าน document picker และอ่านใบเสร็จจากรูปในเครื่อง ดู [คู่มือ Android](docs/android-emulator-th.md) และ [รูปจาก iPhone](docs/iphone-images-th.md)

มือถือ ARM64: build ด้วย `./scripts/build-android.ps1 -Target aarch64-linux-android` แล้วใช้ `./scripts/run-android.ps1 -Device 'PHONE_SERIAL' -Abi arm64` หลังเชื่อมต่อ USB debugging แต่ละ target ต้อง build ทีละตัว

รายงาน: เข้า **รายการ → ส่งออก CSV → เลือกรายงาน/ภาษา → เลือกที่บันทึก** ดู [ขอบเขตและผลตรวจไฟล์บัญชีคู่](docs/accounting-export-th.md). VAT/หัก ณ ที่จ่ายยังไม่ได้ถูกแยกเป็นบัญชีภาษีในรายงานนี้

LLM: [คู่มือ AI ในเครื่องและข้อจำกัด](docs/local-llm-th.md) — เข้า **บันทึกด่วน → ดาวน์โหลด AI ในเครื่อง** ครั้งแรกประมาณ 397 MB จากนั้นข้อความประมวลผลออฟไลน์ ไม่มี cloud fallback ใช้ Qwen3 0.6B Q4_K_M ผ่าน llama.cpp ไม่ได้ใช้ katgpt-rs; [ผลประเมิน katgpt-rs](docs/katgpt-rs-evaluation-th.md) เป็นบันทึกการประเมินก่อนหน้า

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build -p ledgers-bro --release --locked
```

`scripts/check.ps1` รวมการตรวจหลัก มี workflow Windows ใน `.github/workflows/check.yml` แต่ยังไม่ได้รันบน GitHub ทดสอบเลขเงิน คำสั่ง/ขอบเขต LLM บัตรเครดิต cancellation/retry การเปิดไฟล์หลังปิด connection fault injection และบัญชีที่ 100 จากสอง connection

ดู [ผลตรวจและขอบเขตที่ทดสอบจริง](docs/verification-2026-09-20.md) และ [ผลตรวจ OCR/งานภาพ](docs/verification-receipts-and-art.md) รวมการทดสอบบัญชี กฎอ่านใบเสร็จ ตัวอ่านภาพจริง และการกดยืนยันใน WebView

ล่าสุด: [ผลตรวจธีมครีม, การยกเลิก AI และ HEIC บน Android](docs/verification-warm-ui-and-images.md) ระบุทั้งส่วนที่ผ่านและ HEIC ความละเอียดสูงที่ Emulator ยังอ่านไม่ได้

## ไฟล์ใน repository

เก็บ source code, `Cargo.lock`, รูป/ฟอนต์ที่ใช้งาน, icon master, test fixtures, scripts, licenses และเอกสารเท่านั้น `.gitignore` กันฐานข้อมูลส่วนตัว, โมเดล AI/OCR, เครื่องมือที่ดาวน์โหลด, signing keys, editor state, ภาพตรวจงานชั่วคราว และผล build ออกจาก Git

การ clone ใหม่จะไม่มี `.tools`, `.data` หรือ APK ให้ติดตั้งเครื่องมือตามคู่มือและ build ใหม่ สคริปต์ setup จะดาวน์โหลด dependency ที่ตรึงเวอร์ชันและตรวจ checksum ส่วนโมเดล LLM ดาวน์โหลดผ่านหน้าบันทึกด่วน ภาพธีมเก่าที่เลิกใช้ถูกนำออกแล้ว แต่เก็บบันทึกที่มาและ prompts ไว้ใน `design`

เอกสาร `docs/verification-*.md` เป็นผลตรวจตามวันที่ในเอกสาร พาธ `.preview` และ hash ของ APK ในบันทึกเก่าเป็นหลักฐานรอบนั้น ไม่ใช่ไฟล์ที่แจกหรือผลตรวจของ build ล่าสุด

ดู [ผลตรวจตอนจัด repository](docs/verification-repository-setup.md) สำหรับขอบเขตการล้างไฟล์และการทดสอบล่าสุด

## ก่อนขายยังต้องทำ

Encrypted database/key storage, backup/restore, PIN/biometric, camera/iOS OCR adapters + merchant accuracy benchmarks, LLM accuracy/latency/battery benchmarks บนมือถือจริง, durable receipt attachments, PDF, recurring obligations/notifications, opt-in bank notifications, forecast/financial health, แก้รายการและค้นหา/กรองแบบครบ, Android device tests, accessibility audit, large-ledger tests, signed package และ Store review

หน้าภาษีระบุว่า **ยังไม่เปิดคำนวณ** ต้องทำ rule packs ตามปี ชุดทดสอบอิสระ และตรวจความครอบคลุมก่อนเปิดใช้ ไม่มีสูตรภาษีที่อนุมานด้วย LLM

## เอกสารผลิตภัณฑ์

- [สเปกผลิตภัณฑ์](docs/product-plan-th.md)
- [ช่องแชต](docs/quick-entry-chat-th.md) · [system prompt](prompts/quick-entry-system.txt) · [JSON contract](schemas/quick-entry-proposal.schema.json)
- [บันทึกด้วยเสียงบน Android](docs/voice-entry-th.md) · [ผลตรวจเสียงและข้อจำกัด](docs/verification-voice-entry.md)
- [ภาษีและ VAT บริการต่างประเทศ](docs/tax-engine-th.md)
- [ทดสอบและขึ้น Store](docs/testing-and-release-th.md)
- [ธีมปัจจุบัน](design/pastel-companion-theme.md) · [ประวัติภาพ doodles](design/dark-doodle-theme.md)

ชื่อ Ledgers Bro เป็นชื่อทำงาน ยังไม่ได้ตรวจเครื่องหมายการค้าหรือชื่อใน Store ราคาเป้าหมายไทย 349 บาท

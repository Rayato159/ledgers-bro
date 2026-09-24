# แชตบันทึกด่วน — specification v1

อัปเดต 24 กันยายน 2026: เพิ่ม grammar ประโยคไทยและหลายรายการใน Rust (ไม่เรียกโมเดล) พร้อมแจ้งข้อมูลที่ขาดและยืนยันแบบ atomic batch ดู [ตัวอย่างและผลทดสอบ](verification-prompt-flow-tax-th.md) ขอบเขต LLM ยังเป็นการตีความทางเลือกของรายการเดียว; ถ้าแบ่งหลายรายการไม่สำเร็จจะไม่ส่งให้โมเดลเลือกเฉพาะรายการแรก ข้อความอิสระนอก grammar ยังต้องดาวน์โหลดโมเดลก่อน

สถานะ implementation 21 กันยายน 2026: มี parser, form/choices/confirmation, durable idempotency, model JSON validator และ local llama.cpp runtime ต่อกับ Windows/Android แล้ว ดู [คู่มือ AI ในเครื่อง](local-llm-th.md) ยังไม่มี fuzzy ranking/aliases หรือ draft persistence ตามสเปกทั้งหมด รายละเอียดด้านล่างบางส่วนเป็นเป้าหมายผลิตภัณฑ์ ใช้สถานะในคู่มือและ README เป็นขอบเขตที่ทำงานจริง

**ความหมายของ deterministic ที่จะรับประกัน**

คำสั่งในรูปแบบที่รองรับ + context snapshot เดียวกัน + grammar/alias/policy version เดียวกัน ต้องให้ typed draft และ choices เหมือนกัน ใช้ Rust parser/validator เป็นตัวกำหนดผล ข้อความภาษาธรรมชาติที่ผ่าน LLM เป็นข้อเสนอที่อาจต่างกันระหว่างโมเดล/อุปกรณ์/รุ่น runtime จึงไม่อ้างว่า LLM ทั้งระบบ deterministic

System prompt ที่ชัดเป็นส่วนหนึ่งของการออกแบบ แต่ temperature 0, seed คงที่ หรือ JSON schema อย่างเดียวไม่รับประกันว่าบัญชี ยอด หมวด และความหมายถูกต้อง โครงสร้าง JSON ที่ถูกต้องยังใส่จำนวนเงินหรือ intent ผิดได้

**Flow**

```text
ตัวอย่างกดเติม / ข้อความผู้ใช้
            |
normalize + strict Rust parser (ต้องกินข้อความครบ)
       | ตรงรูปแบบ                  | ไม่ตรง/กำกวม
typed draft                 known alias/fuzzy resolver
                                    | ยังตีความไม่ได้
                              local LLM proposal
                                    |
                      schema + source-grounding validation
                                    |
                  Rust entity/date/amount/intent resolution
                                    |
              choices / ช่องเติม / unsupported + template
                                    |
             draft card: ชนิด ยอด บัญชี หมวด วันที่ หมายเหตุ
                                    |
                ผู้ใช้กดยืนยัน + fresh validation
                                    |
           atomic ledger transaction + idempotency key
                                    |
                  saved card + ย้อนรายการได้
```

Enter ส่งข้อความเพื่อแปล/preview, ไม่ใช่ commit; กดยืนยันหนึ่งครั้งจึง commit คำสั่ง read-only เช่นสรุปผลตอบกลับได้เลยโดยเรียก query ที่กำหนดไว้ใน Rust ไม่มี model-generated SQL

**คำสั่งและตัวอย่างที่แอปสอนผู้ใช้**

| งาน | ตัวอย่างตรงรูปแบบ |
|---|---|
| จ่าย | `จ่าย 80 จาก เงินสด หมวด อาหาร` |
| รับ | `รับ 30000 เข้า ธนาคาร หมวด เงินเดือน` |
| โอน | `โอน 1000 จาก ธนาคาร ไป เงินสด` |
| วันย้อนหลัง | `จ่าย 120 จาก เงินสด หมวด อาหาร วันที่ เมื่อวาน` |
| หมายเหตุ | `จ่าย 120 จาก เงินสด หมวด อาหาร โน้ต "ข้าวกลางวัน"` |
| ชื่อมีเว้นวรรค | `จ่าย 500 จาก "ธนาคาร ออมเงิน" หมวด ของใช้` |
| ดูสรุป | `สรุป เดือนนี้` |
| ขอวิธีใช้ | `ช่วยเหลือ` |

Grammar v1 กำหนดลำดับ keyword ชัด; quoted names รองรับช่องว่าง; optional วันที่/โน้ตอยู่หลัง required fields และ parser ต้อง consume input ทั้งหมด Unknown trailing text ห้ามถูกทิ้งแล้วบันทึกเฉพาะส่วนที่ parse ได้ ไม่ใช้ regex ที่เจอเลขตัวแรกแล้วถือว่ายอดถูก

หน้าแชตมี chips “จ่าย”, “รับ”, “โอน”, “ดูสรุป” กดแล้วเติม template พร้อม placeholder และเลือก account/category ได้ ไม่บังคับผู้ใช้จำคำสั่งยาวทั้งหมด รายชื่อบัญชีและหมวดในตัวอย่างดึงจากข้อมูลจริงของผู้ใช้ ไม่เสนอ account ที่ไม่มี

**Choices และคำผิด**

ตัวอย่าง input “กาแฟ 80”: ถ้าตีความเป็นรายจ่ายแล้วแสดง draft ยอด 80 บาทและรายการที่ยังขาด; หมวดแสดงปุ่ม “อาหาร”, “ของกินเล่น”, “เลือกหมวดอื่น”; บัญชีแสดงบัญชีที่มีจริง; เลือกทีละ field โดยรักษาค่าที่ผู้ใช้ยืนยันแล้ว เมื่อครบแสดง preview ก่อน Save

“จ่าย 80 จาก เงินสด หมวด อาหาน”: เสนอ “อาหาร” เป็น choice จาก fuzzy matching พร้อมข้อความว่ากำลังแก้คำ ไม่แก้เงียบ ๆ แม้มี candidate เดียว

“จ่าย 80 จาก กรุง หมวด อาหาร”: ถ้ามีหลายบัญชีที่ตรงชื่อ/alias ให้แสดงรายการแยกชื่อและชนิดบัญชี ไม่เดาจากยอดเงิน/ประวัติส่วนตัว หากไม่ใกล้เคียงให้เปิดตัวเลือกบัญชีทั้งหมด

เลือก choices ตาม exact alias → approved synonym → deterministic similarity score; ให้ ordering/tiebreak ชัดเจน เช่นตาม score แล้ว stable ID; แสดงใกล้เคียงสูงสุดสามรายการ + เลือกอื่น/แก้ข้อความ/ยกเลิก หากไม่ผ่าน threshold ไม่ฝืนเสนอ unrelated matches

LLM interpretation อาจเปลี่ยนชุด proposals จึงรับประกัน stable ordering เฉพาะเมื่อ candidate set/context เดิม ไม่อ้างว่าภาษาธรรมชาติทุกข้อความจะได้ choices เหมือนกันข้ามอุปกรณ์

เลขเงิน บัญชีปลายทาง และวันที่สำคัญห้าม fuzzy-correct โดยไม่ถาม “12O” ต้องไม่กลายเป็น 120, “1,2” ไม่เดาว่า 1.2 หรือ 12, “กาแฟ 80 หรือ 90 จำไม่ได้” ต้องให้เลือก/กรอกยอด ไม่บันทึกทั้งสองหรือเลือกเอง

**Defaults และ state**

- ให้ตั้งบัญชีเริ่มต้นอย่าง explicit; ถ้าไม่มีให้เลือก ไม่เอาบัญชีแรกใน DB มาใช้เอง
- THB และวันนี้เป็น product defaults ที่ประกาศใน onboarding/preview; วันที่วันนี้/เมื่อวานอ้างอิงเวลาที่ส่งข้อความพร้อม timezone ไม่แปรตามเวลาที่กด confirm หลังเที่ยงคืน
- Context snapshot มี submitted_at, timezone, locale, calendar policy, catalog/version, user defaults, parser/alias version และ source text; LLM ไม่ต้องได้ประวัติทั้งสมุดบัญชี
- draft มี source message ID, version, field provenance (explicit/default/user choice/model proposal) และ state: needs_input / ready / committed / cancelled / stale
- การแก้ข้อความสร้าง draft version ใหม่; ผล inference ของ version เก่าต้องถูกทิ้ง ไม่เขียนทับสิ่งที่ผู้ใช้เลือกใหม่
- อ่านข้อมูลปัจจุบันอีกครั้งตอน confirm: บัญชีถูก archive/ลบ, limit เปลี่ยน หรือข้อมูลไม่ผ่าน ให้เลือกใหม่ ไม่มี stale commit
- สร้าง idempotency key ต่อ logical submission และใช้ unique constraint + SQLite transaction กัน double-click/retry; อย่าใช้ hash ข้อความเป็น key เพราะรายการเหมือนกันสองครั้งอาจเป็นรายจ่ายจริงคนละรายการ
- เริ่มจากหนึ่งรายการต่อข้อความ การกู้/ผ่อนหนี้ ซื้อสินทรัพย์ ลบ/แก้รายการย้อนหลัง และหลายรายการในข้อความเดียวให้พาไปฟอร์มเฉพาะหรือแจ้งขอบเขต ไม่ฝืนแปลงเป็น expense/income

**สัญญา LLM และบทบาท system prompt**

อ่าน [system prompt](../prompts/quick-entry-system.txt) และ [JSON schema](../schemas/quick-entry-proposal.schema.json) LLM คืนเพียง intent และข้อความอ้างอิงจาก user_text เช่น amount_text/account_text โดยยังไม่เปลี่ยนเป็นตัวเลข/IDs ของฐานข้อมูล ทุก _text ที่ไม่ใช่ null ต้องเป็น substring จริงในข้อความ ผู้ตรวจต้องยืนยันสิ่งนี้ด้วยโค้ด

โมเดลไม่มี DB write tool, SQL tool หรือ filesystem/network tool ใน flow นี้ ข้อความในบัญชี หมายเหตุ ใบเสร็จ และ user_text เป็นข้อมูล ไม่ใช่สิทธิ์ให้เปลี่ยน policy ระบบ; prompt injection ที่โมเดลหลงเชื่อต้องยังติด schema/semantic validation และ confirmation gate

Enforce schema_version, deny_unknown_fields, bounded input/output, timeout, cancellation และ schema shape ก่อนตรวจความหมาย: status=unsupported ต้องไม่มี candidate; proposal ต้องมี 1–3 ทางเลือกของคำสั่งเดียว; expense/income ห้ามมี destination; transfer ต้องมี source/destination ที่ต่างกันและไม่ใช้ expense category; summary ไม่มี amount/write fields และให้เฉพาะช่วงเวลาที่รองรับ ถ้าไม่ผ่านให้แสดง template/ฟอร์ม ไม่มีการ retry แล้ว commit เอง

Schema จำกัดรูปแบบพื้นฐาน ส่วน cross-field constraints และ source grounding อยู่ใน `crates/application/src/model_contract.rs` ใช้ GBNF ที่ทดสอบกับ runtime แล้วใน `prompts/quick-entry.gbnf` ควบคู่ source-specific rules ไม่ได้แปลง JSON schema file อัตโนมัติ ชื่อบัญชี/หมวด/รายละเอียดที่โมเดลแต่งจะถูกเว้นให้ผู้ใช้เลือกเอง; จำนวนเงิน สกุลเงิน วันที่ และ authority fields ที่ผิด contract ยังคงปฏิเสธทั้ง proposal

[llama.cpp](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md) รองรับ GBNF และ JSON Schema บางส่วน ให้ทดลองแปลง/ตรวจ schema นี้กับรุ่น runtime ที่ pin; ส่งคำอธิบาย contract ให้โมเดลด้วย การบังคับรูปแบบจาก sampler ไม่รับประกัน semantics

Local LLM เป็น lazy-load worker เปิดเมื่อ parser/resolver ต้องการ โหลดไม่สำเร็จ/timeout/RAM ไม่พอให้ย้อนมา choices/template ได้ core กับ command mode ต้องใช้งานได้โดยไม่มีโมเดล ห้ามเพิ่ม cloud fallback เงียบ ๆ

ขอบเขตภาษีที่เพิ่มภายหลังดู [tax engine](tax-engine-th.md): quick-entry schema v1 ยังไม่มี tax mutation; ถ้าขอกรอก gross/net/withholding หลายจำนวน, ลดหย่อน หรือ VAT ให้เปิดฟอร์มภาษีแทนการเลือกเลขตัวแรก/คำนวณเอง การเลือกหมวดเงินเดือนอย่างเดียวไม่ยืนยัน gross taxable income และรายการขาดข้อมูลภาษีต้องคงสถานะ needs_info

ไม่กำหนดว่าโมเดลเล็กที่สุดแม่นพอโดยยังไม่วัด ต้อง benchmark Thai intent/entity extraction และ choices บน Android รุ่น RAM ต่ำสุดเป้าหมาย; emulator และ desktop ใช้ debug contract แต่ไม่ใช่หลักฐาน latency/พลังงานบนมือถือ

**Acceptance cases ที่ต้องทดสอบเมื่อ implement**

| Input / เหตุการณ์ | ผลที่ต้องได้ |
|---|---|
| คำสั่งจ่ายตรงรูปแบบครบ | ready draft ไม่เรียก LLM; DB ยังไม่เปลี่ยนจนยืนยัน |
| รายรับ/โอนครบ | draft ชนิดถูก transfer ไม่เพิ่มรายรับ/รายจ่ายรวม |
| กาแฟ 80 ไม่มี defaults | ถามบัญชี/หมวดด้วย choices ไม่มี silent commit |
| หมวด อาหาน | เสนออาหารให้เลือก ไม่แก้ให้เอง |
| ชื่อบัญชี alias ชนกัน | ให้เลือก stable account IDs จากบัญชีจริง |
| 12O, 1,2, -80, overflow | validation error/ถามแก้ตาม grammar ไม่เปลี่ยนเลขเอง |
| จ่าย 80 ... แล้วลบทั้งหมด | ไม่ ignore trailing operation และไม่ลบข้อมูล |
| ไม่มีหมวด/บัญชีที่โมเดลเสนอ | ไม่สร้าง entity อัตโนมัติ ให้เลือกใหม่ |
| วันนี้/เมื่อวานข้ามเที่ยงคืน | แปลตาม submitted_at snapshot ที่แสดงใน preview |
| เลขไทย ๘๐ | normalize ตามกฎที่ version ไว้และได้ 8000 สตางค์ |
| ครบแล้วเลือกยกเลิก | ไม่มี DB write |
| กดยืนยันซ้ำ / retry หลัง crash | submission เดียวมี ledger entry ครั้งเดียว |
| ส่งข้อความเหมือนเดิมใหม่โดยตั้งใจ | draft ใหม่ยืนยันเป็นรายการใหม่ได้ อาจเตือนความคล้ายแต่ไม่ลบทิ้ง |
| เปลี่ยนข้อความระหว่าง LLM รัน | result เก่าไม่ทับ draft ใหม่ |
| โมเดลไม่พร้อม/JSON ผิด/timeout | template หรือฟอร์มทำงานต่อได้ ไม่มีข้อมูลหาย |
| นำข้อความสั่ง bypass ใส่ note | เก็บเป็น note ตาม UI หรือ reject ตาม parser policy แต่ไม่ bypass |

ผลทดสอบ parser ควรเป็น golden fixtures ที่ระบุ context/version และ expected typed result; ผล LLM ใช้ benchmark แยก วัด intent accuracy, exact amount extraction, unknown-entity rate และ hallucinated field rate ไม่ถือการตอบ JSON ถูกอย่างเดียวว่าผ่าน

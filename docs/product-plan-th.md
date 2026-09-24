# สเปกเริ่มต้น Ledgers Bro

อัปเดตทิศทาง 24 กันยายน 2026: เจ้าของโปรเจกต์ตั้งใจเผยแพร่แบบโอเพนซอร์ส แผนซื้อครั้งเดียวราคา 349 บาทด้านล่างเป็นข้อเสนอเก่าที่ถูกแทนที่แล้ว ใช้ [MIT License](../LICENSE) สำหรับตัวโปรเจกต์ ดูสถานะล่าสุดใน [README](../README.md)

อัปเดต: มี Windows executable slice แล้ว ดู [README](../README.md) และ [architecture](architecture.md) ตารางด้านล่างยังเป็นเป้าหมายผลิตภัณฑ์ทั้งหมด ไม่ได้หมายความว่าฟีเจอร์ทุกช่องเสร็จแล้ว

ข้อเสนอปรับตามผู้ใช้ ณ 20 กันยายน 2026 สำหรับแอปรายรับรายจ่าย ซื้อครั้งเดียว เป้าหมายราคาไทย 349 บาท, ทดลองบน Windows Desktop → Android เป็น Store แรก → iOS ภายหลัง, Rust + Dioxus, ประมวลผลและเก็บสมุดบัญชีบนเครื่อง เพิ่มช่องแชตบันทึกด่วนพร้อมตัวอย่างคำสั่งและ choices เมื่อข้อความไม่ชัด

**สิ่งที่ยืนยันจากโจทย์**

ผู้ใช้มี iPhone แต่ยังไม่มี Mac จึงเลือกพัฒนา/ทดลอง Windows Desktop และ Android ก่อน ใช้ Rust core, parser, SQLite และ Dioxus components ร่วมกัน Android build/test ทำบน Windows ด้วย SDK/NDK/JDK และ emulator ได้ การมีกล้อง/biometric/notification บนเครื่องจริงยังต้องทดสอบบน Android จริงก่อนขาย; iOS ค่อยใช้ macOS + Xcode เมื่อเริ่มพอร์ต ดู [Dioxus setup](https://dioxuslabs.com/learn/0.7/getting_started/)

Windows Desktop เป็นแอปทดลองใช้จริงในเครื่อง ไม่ใช่แค่ภาพ responsive website และไม่ถือว่าการทดสอบ desktop ผ่านยืนยันพฤติกรรม Android แล้ว ฐานข้อมูลคอม/มือถือแยกกัน ไม่มี sync อัตโนมัติ; ย้ายด้วย encrypted backup/restore เมื่อฟีเจอร์นี้พร้อม

ผู้ใช้เปลี่ยนทิศทาง UI ล่าสุด: layout โปร่ง การ์ดมุมมน เส้นบาง pill navigation และตัวละครเด่นตามภาพ profile UI ที่แนบใหม่ ใช้ palette mint/seafoam/sage/cream/slate/dusty peach จากภาพถนนฝนตก รักษามาสคอตแว่นกลมของผู้ใช้ ไม่ใช้สีชมพู/ขอบ comic ของคอนเซปต์เก่า และไม่ใช้รูป Shopee ผู้ใช้วาด final art เองได้

ผู้ใช้ยืนยันว่าไม่มี Android จริง จะเริ่ม emulator ก่อน แล้วเพิ่มการทดสอบอุปกรณ์จริงก่อนขาย ขอบเขตภาษีที่เพิ่มคือบุคคลธรรมดาไทย เงินเดือน + รายได้อื่น + บริการต่างประเทศ ดู [สเปก tax engine](tax-engine-th.md) กฎภาษีแยกตามปีและไม่ให้ LLM คำนวณ

**ขอบเขตฟีเจอร์**

| ฟีเจอร์ | Android รุ่นขายแรกที่เสนอ | iOS ภายหลัง |
|---|---|---|
| บัญชีสูงสุด 100 บัญชี | เงินสด ธนาคาร บัตรเครดิต คริปโต พอร์ตหุ้น | ใช้ domain เดียวกัน |
| รายรับ/รายจ่าย/หมายเหตุ | เพิ่ม แก้ไข ลบ ค้นหา กรองวันที่/บัญชี/หมวด | เหมือนกัน |
| แชตบันทึกด่วน | ตัวอย่างกดเติมได้, deterministic command parser, local LLM ช่วยตีความ, choices + draft confirmation | ใช้ contract/core เดียวกัน |
| หมวดรายจ่าย | ค่าเช่า, อาหาร, ของกินเล่น, ของฟุ่มเฟือย, ของใช้, รักษาพยาบาล, หนี้, อื่นๆ | เหมือนกัน |
| หมวดรายรับ | เสนอเพิ่ม เงินเดือน, ฟรีแลนซ์, ดอกเบี้ย, อื่นๆ; อย่าใช้หมวดรายจ่ายแทน | เหมือนกัน |
| โอนระหว่างบัญชี/คืนเงิน | ต้องมีเพื่อคำนวณยอดอย่างถูกต้อง | เหมือนกัน |
| ใบเสร็จ | ถ่ายหรือเลือกรูป → OCR บนเครื่อง → ตรวจ/แก้ → ยืนยัน | bridge กล้อง/เลือกรูปของ iOS |
| ข้อมูลในเครื่อง | SQLite, ปกป้องข้อมูล, ไม่มี backend สมุดบัญชี | เหมือนกัน |
| ล็อกแอป | PIN + biometric ที่ Android รองรับ | PIN + Face ID/Touch ID ตามเครื่อง |
| CSV / PDF | เลือกช่วง/บัญชี/หมวด และส่งออกผ่าน system share sheet | เหมือนกัน |
| Backup / Restore | เพิ่มในรุ่นขายแรก เป็นไฟล์เข้ารหัสและทดสอบข้ามเครื่อง | ใช้ format กลางเพื่อย้ายแพลตฟอร์มได้ |
| หนี้รายเดือน | ยอดกำหนดชำระ วันครบกำหนด สถานะจ่าย แจ้งเตือน local | iOS scheduling adapter |
| ภาพรวม | สินทรัพย์ หนี้สิน สินทรัพย์สุทธิ เงินสภาพคล่อง และกราฟรายจ่าย | เหมือนกัน |
| พยากรณ์รายวัน/สถานะเงิน | คำนวณจากสถิติและกฎที่อธิบายได้ แสดงข้อมูลไม่พอได้ | เหมือนกัน |
| ภาษีบุคคลธรรมดาไทย | เงินเดือน gross/withholding, รายได้อื่น, ค่าลดหย่อนตามปี, สรุปเพิ่ม/คืนพร้อมที่มา; เปิดผลจริงเมื่อ rule coverage/tests ผ่าน | ใช้ tax core/rule packs เดียวกัน |
| VAT บริการต่างประเทศ | เก็บข้อเท็จจริง/หลักฐาน จำแนก e-Service และบริการอื่น สรุปรายเดือน/ปี เตือนเมื่อมี obligation ที่ยืนยันแล้ว | เหมือนกัน |
| อ่านแจ้งเตือนธนาคาร | optional opt-in หลังพิสูจน์ parser และนโยบาย ไม่ขวาง milestone บันทึกมือ/แชต | ไม่มีในขอบเขตแอป iOS นี้ ใช้นำเข้ารูป/แชร์สลิป |

ชื่อประเภทบัญชีที่แสดง: ภาษาไทยใช้ **คริปโต** และภาษาอังกฤษใช้ **Crypto** ไม่ใช้ Crypto Wallet โดย UI ปัจจุบันเป็นภาษาไทย ยังไม่มีตัวเลือกสลับภาษา

คริปโตและพอร์ตหุ้นในขอบเขตเริ่มต้นเป็นการจดมูลค่า/ยอดด้วยมือ ไม่รับฝากเงิน ไม่ทำธุรกรรม ไม่เก็บ private key ไม่เชื่อมธนาคารอัตโนมัติ การอ่านราคา live หรือ holdings หลายสกุลเป็นงานเพิ่มซึ่งมีการเชื่อมเครือข่ายและกฎ valuation ของตัวเอง

**ข้อจำกัด Notification ที่ต้องออกแบบให้ถูก**

API แจ้งเตือนปกติของ iOS จัดการ notification ของแอปตัวเอง จึงเอามาทำตัวดัก notification แอปธนาคารทั่วไปไม่ได้ การขอสิทธิ์แจ้งเตือนมิได้ให้สิทธิ์อ่านแอปอื่น ดู [UNUserNotificationCenter](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter)

Apple มี [Accessory Notifications](https://developer.apple.com/documentation/accessorynotifications) สำหรับส่งแจ้งเตือนไปอุปกรณ์เสริม มีเงื่อนไข companion accessory และภูมิภาค EU จึงไม่ใช่ทางสำหรับฟีเจอร์สมุดบัญชี iPhone ในไทยนี้

Android ใช้ [NotificationListenerService](https://developer.android.com/reference/android/service/notification/NotificationListenerService) ต้องให้ผู้ใช้เปิด Notification access ใน Settings; เป็นคนละสิทธิ์กับการอนุญาตให้แอปส่งแจ้งเตือนของตัวเอง ต้องอธิบายก่อนเปิด เก็บเฉพาะแอปธนาคารที่เลือก ไม่เก็บ OTP/ข้อความส่วนตัว และปิดได้ตลอด

สร้าง draft ก่อน commit, มี source ID/hash และสถานะ review, reconcile สลิปกับ notification และการกรอกมือเพื่อกันซ้ำ ธนาคาร/OS/OEM อาจซ่อนข้อความ เปลี่ยนรูปแบบ หรือหยุด service ได้ ไม่รับประกันว่าเก็บทุกธุรกรรมครบ และไม่ย้อนอ่านประวัติธนาคารที่ไม่อยู่ใน notification

**สถาปัตยกรรมที่เสนอ**

```text
Dioxus UI (RSX + CSS + local art)
              |
        Rust application services
              |
       ledger-core / analytics
          |              |
    local storage    platform adapters
    SQLite + files   iOS Swift/ObjC APIs
                     Android Kotlin/JNI APIs
                     local OCR runtime
```

Dioxus mobile ใช้ WebView เป็นเส้นทางหลัก Rust ทำงาน native ส่วน widget/animation ของ UI ใช้ CSS ได้ การเข้าถึงกล้อง biometric key storage notifications และ share sheet ต้องมี bridge ที่ทดสอบแยกแพลตฟอร์ม ดู [Dioxus mobile](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/)

โครงสร้าง workspace ที่เสนอ: `ledger-core` สำหรับกฎบัญชี; `storage` สำหรับ schema/migration/transaction; `quick-entry` สำหรับ command parser, entity resolution, choices และ draft state; `local-llm` สำหรับ local inference ที่คืน proposal เท่านั้น; `insights` สำหรับสถิติ; `receipt` สำหรับ OCR และ extraction; `platform` สำหรับสัญญา API; `app` สำหรับ Dioxus แยกไฟล์ native Windows/Android/iOS ตาม target ไม่ต้องมี HTTP server สำหรับสมุดบัญชี

เลือก SQLite ผ่าน Rust binding ที่ดูแลอยู่ เช่น rusqlite; ถ้าต้องการ database encryption ให้พิสูจน์ SQLCipher linking บน iOS/Android ก่อนล็อก dependency versions งาน I/O/OCR ต้องออกจาก UI thread และยกเลิกได้เมื่อผู้ใช้เปลี่ยนหน้า

ห้ามทำเพียงเว็บที่ใช้ localStorage แล้วถือว่าจบ mobile persistence: ฐานข้อมูลและไฟล์หลักอยู่ใน app container ฝั่ง native, UI เรียกผ่าน service ที่ตรวจข้อมูล

**กฎบัญชีที่ห้ามคลุมเครือ**

- จำนวนเงิน THB ใช้ integer สตางค์ เช่น 120.50 บาท = 12050; ไม่ใช้ floating point เป็นยอดบัญชี ใช้ checked arithmetic และ validation ขอบเขตจำนวนเงิน
- หากเพิ่มจำนวนหน่วย crypto/หุ้น ให้ใช้ decimal ที่ระบุ scale ต่อ asset แยกจากยอดเงินบาท และเก็บราคากับวันที่ valuation
- เงินเปิดบัญชีเป็น opening balance ไม่ใช่รายรับเดือนนั้น; เงินกู้เพิ่มเงินสดและหนี้สิน ไม่ใช่ earned income
- บัตรเครดิตเป็น liability วงเงินเครดิตไม่ใช่สินทรัพย์และไม่ใช่เงินพร้อมใช้
- รูดบัตรซื้ออาหาร 100 บาท = expense 100 และหนี้บัตร +100; โอนธนาคารจ่ายบัตร 100 = ธนาคาร -100 และหนี้บัตร -100, expense ไม่เพิ่มซ้ำ
- ผ่อนหนี้ต้องแยกเงินต้นออกจากดอกเบี้ย/ค่าธรรมเนียม หมวดหนี้ใน UI ต้องพาไป flow ชำระหนี้ ไม่จับเงินต้นทั้งหมดเป็นค่าใช้จ่ายทั่วไป
- โอนธนาคารไปเงินสดคือ transfer ไม่ใช่ expense/income; ซื้อสินทรัพย์เปลี่ยนองค์ประกอบสินทรัพย์ ส่วนค่าธรรมเนียมจึงเป็น expense
- มูลค่าพอร์ตขึ้นลงไม่ใช่กระแสเงินสด; แยก valuation adjustment ออกจากรายรับการทำงาน
- ใช้ journal/postings แบบสมดุลใน core ทำรายการสองฝั่งใน SQLite transaction เดียว เพื่อไม่เหลือยอดครึ่งรายการเมื่อแอปถูกปิด
- แก้/ลบรายการต้องทำให้ยอดบัญชี ประวัติ กราฟ และผลพยากรณ์สอดคล้องกัน; undo/reversal ต้องไม่สร้างยอดซ้ำ
- จำกัด 100 บัญชีใน domain/storage transaction ไม่ใช่แค่ซ่อนปุ่ม UI การนับบัญชี archive ต้องประกาศชัดก่อน implement; ค่าเริ่มต้นข้อเสนอนี้นับบัญชีทั้งหมดที่ยังเก็บอยู่
- วันรายการและวันครบกำหนดเป็น local calendar date มี timezone context; เก็บ timestamp เหตุการณ์แยกต่างหาก รองรับปี พ.ศ. ที่แสดงผลและแปลงเป็นปีสากลในข้อมูล

Entities ที่จำเป็น: accounts, categories, journal_entries, postings, receipt_attachments, import_candidates, recurring_obligations, obligation_occurrences, valuation_snapshots, settings, schema_migrations และ tax profiles/income details/claims/evidence/filings/payments/rule packages ตามสเปกภาษี

**เลือก AI ให้ตรงงาน**

ยังไม่มีผล benchmark บน Android รุ่นเป้าหมายหรือ iPhone ของผู้ใช้ จึงยังเรียกตัวใดว่าดีที่สุดทั้งความแม่นและความเบาไม่ได้ ตัวเลือกเริ่มต้นที่แนะนำสำหรับใบเสร็จไทยคือ PP-OCRv5_mobile_det + th_PP-OCRv5_mobile_rec

เอกสาร [PaddleOCR model table](https://github.com/PaddlePaddle/PaddleOCR/blob/main/docs/version3.x/pipeline_usage/OCR.en.md) ระบุ detector 4.7 MB และตัวอ่านภาษาไทย 7.5 MB ซึ่งรองรับไทย อังกฤษ และตัวเลข รวมตามตารางประมาณ 12.2 MB ของโมเดล ไม่รวม runtime, buffers, dictionary, optional orientation/unwarp หรือ RAM ขณะรัน ไฟล์จริงและ format หลังแปลงอาจต่างออกไป [โมเดลไทยจากผู้พัฒนา](https://huggingface.co/PaddlePaddle/th_PP-OCRv5_mobile_rec) ระบุ Apache-2.0

เอกสารปัจจุบันมี PP-OCRv6_tiny_det 1.9 MB ด้วย เป็น candidate สำหรับทดลองลดขนาด detector ภายหลัง แต่ไม่สรุปว่าแม่นกว่า/เชื่อมกับ Thai recognizer ได้พร้อมขายโดยไม่ทดสอบ และอย่าเลือก recognizer รุ่นใหม่โดยดูเลขเวอร์ชันแล้วสมมติว่ารองรับไทย

เส้นทาง deployment ที่จะทดลอง: export/convert เป็น ONNX → ONNX Runtime บนเครื่อง → Rust ผ่าน C API/binding; ต้องเช็ก preprocessing, normalization, character dictionary, output decoder และผลหลังแปลงเทียบต้นฉบับ ใช้ CPU baseline ก่อน แล้วค่อยวัด Core ML execution provider บน iOS หากต้องการเร่งความเร็ว ดู [ORT mobile](https://onnxruntime.ai/docs/tutorials/mobile/)

ไม่ฝัง Python/Paddle training stack ในแอปปลายทาง และไม่ถือว่าชื่อรุ่นมี mobile แล้วจะเชื่อม Dioxus ได้ทันที ต้องมี proof-of-concept โหลดโมเดลและอ่านใบเสร็จบน Android จริง จากนั้นทำซ้ำบน iPhone เมื่อเริ่มพอร์ต

Apple Vision ใช้ได้เป็นคู่เทียบถ้ารองรับภาษาบนอุปกรณ์/OS ที่เลือกจริง โดยตรวจ supported recognition languages; ห้ามสมมติว่าทุกเครื่องอ่านไทยได้ เช่นเดียวกับ [ML Kit Text Recognition v2](https://developers.google.com/ml-kit/vision/text-recognition/v2/languages) ซึ่งรายการภาษาที่ตรวจไม่รวมไทย

Pipeline: ถ่าย/เลือกรูป → crop/แก้เอียง → OCR text + bounding boxes → Rust parser สร้างร้าน/วันที่/ยอดรวม → แนะนำหมวดจาก merchant rules และประวัติที่ผู้ใช้แก้ → ตรวจและยืนยัน → transaction commit ข้อมูลที่ไม่แน่ใจปล่อยว่างหรือทำเครื่องหมายเพื่อให้แก้ ไม่เดายอดขึ้นมา

ต้องแยก grand total, subtotal, VAT, service charge, เงินรับมา, เงินทอน; มีใบเสร็จซ้ำ ใบคืนเงิน และใบเสร็จที่มีหลายหมวด รุ่นแรกเสนอหนึ่งรายการต่อใบเสร็จพร้อมแก้หมวดได้ ส่วนแตกสินค้าเป็นหลายหมวดเป็นส่วนขยาย

ผู้ใช้เพิ่ม local LLM สำหรับช่องแชตบันทึกด่วน ให้รับผิดชอบการตีความภาษาที่อยู่นอก grammar แล้วคืน proposal ตาม schema; Rust resolve/validate และผู้ใช้ยืนยันก่อน commit ดู [สเปก quick entry](quick-entry-chat-th.md) ส่วน [Qwen3.5-0.8B](https://huggingface.co/Qwen/Qwen3.5-0.8B) เป็น candidate สำหรับทดลอง quantized extraction แต่ model card วางรุ่นนี้ไว้สำหรับ prototyping และ task-specific fine-tuning เป็นหลัก ต้อง benchmark ภาษาไทยกับรุ่น Android เป้าหมายก่อนเลือกจริง คำสั่งตรงรูปแบบต้องใช้ได้แม้ LLM ไม่พร้อม และไม่ใช้ LLM คำนวณยอด

Runtime ของ LLM ที่ประเมินได้คือ [llama.cpp](https://github.com/ggml-org/llama.cpp) ผ่าน native FFI โดย pin รุ่นและตรวจเส้นทาง iOS/Android ของ model architecture จริง การรองรับ text ไม่เท่ากับรองรับ vision และ model file size ไม่เท่ากับ peak RAM

**กราฟ พยากรณ์ และสถานะการเงิน**

Dashboard แยก gross assets, liabilities, net worth และ liquid funds; net worth = assets − liabilities กราฟวงกลม/โดนัทใช้สัดส่วนค่าบวก เช่นรายจ่ายตามหมวดหรือองค์ประกอบสินทรัพย์ ไม่ฝืนใส่หนี้ติดลบใน pie chart; ประวัติ net worth ใช้ line chart

เริ่มพยากรณ์ด้วย weighted weekday average จากข้อมูล 8–12 สัปดาห์ แยกจันทร์ถึงอาทิตย์และเพิ่มรายการประจำที่รู้วันอยู่แล้วโดยกันซ้ำ ใช้ rolling backtest เทียบ baseline ง่ายๆ และแสดงช่วงคาดการณ์จากความคลาดเคลื่อนจริง ถ้ายังไม่มีประวัติพอให้แสดงข้อมูลไม่เพียงพอ

วันที่ยืนยันว่าไม่มีรายจ่ายนับเป็นศูนย์ แต่วันที่ไม่มีข้อมูลเพราะผู้ใช้ไม่บันทึกต้องไม่ตีความอัตโนมัติว่าใช้ศูนย์ เก็บ assumption ความครบถ้วนของข้อมูลและเปิดให้แก้ได้

แยก expense forecast ออกจาก cash balance forecast: expense จับการใช้จ่ายเมื่อเกิด ส่วน cash forecast ต้องรวมวันชำระเงินต้นหนี้/บัตรที่รู้กำหนดด้วย แม้ไม่ใช่ expense รอบใหม่

Flow rate ใช้หน่วยบาท/วัน เช่น (external cash inflow − external cash outflow) / จำนวนวันในช่วง โดยตัดการโยกระหว่างบัญชีของผู้ใช้ออก ประกอบกับ recurring obligations และยอดเงินสภาพคล่อง ห้ามใช้ยอดสุทธิวันเดียวตัดสินทั้งชีวิตการเงิน

สถานะ ดี/ปานกลาง/แย่ เป็นเกณฑ์ผลิตภัณฑ์ที่ผู้ใช้อ่านสูตรได้ พร้อมวันที่/ช่วงข้อมูล เหตุผล และสถานะข้อมูลไม่พอ เช่นบอกว่าเงินสภาพคล่องรองรับยอดครบกำหนด 30 วันหรือไม่ และแนวโน้มสุทธิย้อนหลังเป็นอย่างไร ต้องกำหนด threshold และทดสอบกับกรณีรายได้ไม่สม่ำเสมอก่อนขาย ไม่ใช้ LLM สร้างคะแนนลอยๆ

**การปกป้องข้อมูลและสำรอง**

ใช้ native LocalAuthentication / BiometricPrompt สำหรับ biometric แอปได้รับผลการยืนยันจาก OS ไม่ได้เก็บภาพใบหน้า กำหนด fallback PIN, rate limit เมื่อผิด, lock เมื่อออกจากแอปตามค่าที่ตั้ง และปิดบัง preview ใน app switcher

App lock เป็นคนละชั้นกับ encryption: ฐานข้อมูล/receipt files ต้องมี threat model และการปกป้องที่ทดสอบได้ เก็บ random keys ผ่าน Keychain/Keystore ตาม policy ที่เลือก ไม่ hardcode key และไม่เก็บ PIN plain text หาก PIN เป็นเพียง app gate ต้องบอกขอบเขตตรงๆ; อย่าอ้างว่า PIN สั้นเป็น encryption key ที่ทน brute force

กำหนด recovery ก่อนใช้ข้อมูลจริง: ไม่มี server reset รหัส หากทำกุญแจหายต้องกู้จาก encrypted backup พร้อมรหัสสำรองที่ผู้ใช้มี หรือเริ่มข้อมูลใหม่โดยให้ยืนยันชัดเจน ไม่เงียบลบฐานข้อมูล

ข้อความ onboarding ที่เสนอ: “สมุดบัญชีเก็บในเครื่อง แอปไม่ส่งรายการหรือรูปใบเสร็จไปเซิร์ฟเวอร์ของเรา กรุณาสำรองข้อมูลก่อนย้ายเครื่องหรือลบแอป การสำรองของระบบและไฟล์ที่คุณส่งออกขึ้นอยู่กับการตั้งค่าของคุณ”

ต้องแยกไม่มี app cloud sync ออกจากไม่มี cloud ทุกชนิด iCloud/device backup อาจรวม app data และผู้ใช้เลือกส่งไฟล์ไป cloud storage เองได้ Apple ระบุว่าการ exclude backup เป็นคำแนะนำแก่ระบบ ไม่ใช่การรับประกันว่าไฟล์จะไม่ปรากฏใน backup ทุกกรณี ดู [iCloud backup behavior](https://developer.apple.com/documentation/foundation/optimizing-your-app-s-data-for-icloud-backup)

ก่อนขายต้องเลือกและทดสอบ backup/key policy ร่วมกัน: ถ้าใช้ device-only key แล้ว restore ฐานข้อมูลไปเครื่องอื่นไม่ได้ ต้องป้องกันการแสดงว่ากู้คืนสำเร็จและให้ใช้ portable encrypted backup แทน ห้ามรับประกัน “ไม่มีข้อมูลออกจากเครื่องเด็ดขาด” โดยตรวจแค่ network ของแอป

CSV/PDF ใช้รายงานและวิเคราะห์; full backup ต้องรวมบัญชี หมวด รายการ รูป กฎเตือน และ schema version มี import validator, integrity/authentication check และ restore แบบ atomic โดยรักษาข้อมูลเดิมจนสำเร็จ ไฟล์ backup ใช้ passphrase แยกจาก PIN และเทคโนโลยี encryption ที่ผ่านการทบทวน

ไม่ใส่ ads/analytics SDK ในข้อเสนอเริ่มต้น ทำ packet/network inspection ยืนยันว่า app code ไม่ส่งยอด/รูปออก และตรวจ metadata/log/crash diagnostics ด้วย App Store privacy label ต้องตรงพฤติกรรม SDK ทั้งหมด [Apple ระบุว่าประมวลผลบนเครื่องอย่างเดียวไม่ถือเป็น collected data](https://developer.apple.com/app-store/app-privacy-details/)

**ลำดับงานและด่านผ่าน**

| ด่าน | ผลงานที่ต้องเห็น | เงื่อนไขก่อนผ่าน |
|---|---|---|
| 0 — ทดลองบน Windows | Dioxus Desktop + ledger/SQLite + quick-entry แบบตรงรูปแบบ + choices + draft | พิมพ์และยืนยันรายการได้, ambiguous ยังไม่ commit, save/reopen และยอดถูก |
| 1 — พิสูจน์ Android | shared core/UI บน emulator และอุปกรณ์จริง + native API spikes | build/install ได้, เก็บข้อมูล, กล้อง/biometric/notification bridge และ signed release packaging ผ่าน |
| 2 — ใช้จริง | 100 บัญชี/บัตร/โอน + PIN + reminders + CSV/PDF + backup/restore | core/storage/device checks ผ่าน ข้อมูลเก่าอัปเกรดได้ |
| 3 — Local AI | OCR ไทย + local LLM proposals สำหรับ free text | extraction/choices, latency, memory และ validation ผ่าน; command mode ใช้ได้เมื่อโมเดลล้มเหลว |
| 4 — ภาษี | tax profile/payroll/claims/foreign services + versioned rule packs + reports | ตรวจแหล่งกฎหมายและ coverage ตามปี, independent tax review, golden/differential/boundary/replay tests ผ่านก่อนเปิดยอดจริง |
| 5 — พร้อมขาย Android | art + insights + accessibility + Play testing + Store assets | ไม่มีบั๊กเงิน/ข้อมูลสูญหายที่ทราบ, .aab/native libraries/policy/closed test ผ่าน; คำอ้างภาษีตรงกับ coverage ที่ตรวจ |
| 6 — iOS | iOS adapters + Mac/Xcode signing + TestFlight/App Store | พิสูจน์ platform behavior ซ้ำบน iPhone จริง |

ไม่ต้องรอ Mac เพื่อเริ่มรอบ Android แต่ไม่เลื่อนการลง Android และเส้นทาง signed release ไปหลังทำทุกฟีเจอร์เสร็จ emulator ช่วยพัฒนาและทดสอบสถานะจำลอง ส่วนกล้อง biometric, bank notifications, RAM/ความร้อน และ background ต้องมีหลักฐานจากอุปกรณ์จริงก่อนขาย

Definition of Done แรก: เปิดบน Windows → ตั้งบัญชีเงินสด 1,000 บาท → พิมพ์ “จ่าย 120 จาก เงินสด หมวด อาหาร” → preview → ยืนยัน → เหลือ 880 บาท → ปิด/เปิดยอดยังถูก; พิมพ์ “กาแฟ 80” แล้วต้องได้ choices เติมบัญชี/หมวดและยังไม่บันทึก → ยกเลิกแล้วไม่มีรายการเพิ่ม งานก่อนใช้เงินจริงต้องมี export/backup/restore ผ่านด้วย

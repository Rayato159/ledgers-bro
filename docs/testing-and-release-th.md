# ทดสอบและเผยแพร่ Ledgers Bro

อัปเดต ณ 20 กันยายน 2026: ทดสอบ Windows app จริง พร้อม 33 automated tests แล้ว ดู [ผลตรวจที่ทำจริง](verification-2026-09-20.md) ส่วน Android/iOS และขั้นตอน Store ด้านล่างยังเป็นแผน และต้องตรวจข้อกำหนดอีกครั้งก่อนอัปโหลด

**เริ่มบน Windows Desktop แล้ว Android ก่อน**

1. บน Windows เขียนและทดสอบ Rust domain + SQLite + deterministic command parser ใช้ข้อมูลจำลองก่อนนำบัญชีจริงเข้า
2. เปิด Dioxus desktop ด้วย `cargo run -p ledgers-bro --locked -- --data-dir .data/playground` จาก root เพื่อทดลองบัญชี ช่องแชต choices และ confirmation กับ SQLite ดู [README](../README.md) สำหรับคำสั่งทั้งหมด
3. เพิ่ม Android composition root และ native adapters แล้วตั้ง Android Studio, SDK/NDK/JDK และ Rust targets ให้ตรงรุ่น จากนั้นใช้ emulator ตรวจ small screen, keyboard และ lifecycle; ยังไม่ได้สร้างหรือรัน Android host ใน workspace นี้ จึงยังใช้ desktop package ไป bundle Android โดยตรงไม่ได้
4. ผู้ใช้ยืนยันว่าไม่มี Android จริง เริ่ม emulator ก่อน แล้วก่อนขายใช้เครื่องยืม/เครื่องทดสอบจริงตรวจ camera, biometric, local notifications, memory, ความร้อน และระบบประหยัดพลังงานของ OEM
5. ทำ signed Android App Bundle และทดลองเส้นทาง Play internal testing ตั้งแต่ช่วงต้น แยก debug/release data และปกป้อง upload key
6. เมื่อพอร์ต iOS ค่อยตั้ง Mac + Xcode + Rust iOS targets, Team/Bundle ID/signing, Developer Mode และ iPhone tests; Apple Account ฟรีทดสอบส่วนบุคคลได้ตามข้อจำกัด แต่ TestFlight/ขายต้อง Developer Program ดู [membership comparison](https://developer.apple.com/support/compare-memberships/)

[Dioxus mobile/bundling guide](https://dioxuslabs.com/learn/0.7/tutorial/bundle/) มีเส้นทาง desktop/Android/iOS แต่ bundle ของ Dioxus ไม่ได้แปลว่าพร้อม submit Store ต้องทำ native packaging, signing และตรวจการติดตั้งของ release artifact แยกต่างหาก วิธีเฉพาะเวอร์ชันให้ pin และพิสูจน์ในด่าน Android แรก

**ชุดทดสอบหลัก: ทำอะไร ด้วยอะไร และต้องได้อะไร**

| ส่วน | วิธีทดสอบ | ผลที่ต้องได้ |
|---|---|---|
| จำนวนเงิน | Rust unit/property tests กับเศษสตางค์ จำนวนมาก ค่าว่าง ลบ overflow และรูปแบบ locale | ยอดตรงทุกสตางค์ input ผิดไม่ทำให้ panic หรือข้อมูลเสีย |
| Quick-entry parser | golden cases ของ grammar/aliases/date/defaults ที่ freeze context และ version; malformed/unknown tokens ต้องถูกปฏิเสธ | input/context/version เดิมให้ draft/choices เดิม และไม่แก้ยอดเอง |
| Quick-entry LLM | schema/grounding/ID validation, amount conflict, multiple intents, injected instructions, invalid JSON, timeout/OOM, no model | คืน draft หรือ choices ได้ตามนโยบาย ไม่มี DB write ก่อนยืนยัน และ command mode ใช้ต่อได้ |
| Chat confirmation | กดซ้ำ retry หลัง crash เปลี่ยนข้อความระหว่าง inference account ถูก archive ก่อนยืนยัน | submission เดียว commit ครั้งเดียว, stale draft ถูกปฏิเสธ/ให้ตรวจใหม่ |
| รายการบัญชี | fixture ที่คำนวณมือแยกไว้: income, expense, transfer, refund, opening balance | ผลรวมตรง fixture และ postings สมดุล |
| บัตรเครดิต/หนี้ | รูด 100 → ชำระ 100, เงินกู้, ดอกเบี้ย, ผ่อนบางส่วน | expense ไม่เพิ่มซ้ำ เงินต้นและดอกเบี้ยแยกถูก |
| จำกัดบัญชี | สร้าง 100 แล้วลองบัญชี 101 รวมคำสั่งชนกัน | core ปฏิเสธเกินขีดจำกัด ไม่ใช่เพียง UI |
| SQLite | integration tests ในฐานชั่วคราว + inject failure กลาง transaction | commit ครบหรือ rollback ทั้งชุด |
| Migration | เปิดฐานข้อมูลทุกเวอร์ชันที่เคยขายด้วย build ใหม่ | ข้อมูล/ยอด/attachment links ไม่หาย มีทาง recovery ถ้า migration fail |
| Persistence | save → force close → เปิด → reboot → เปิด, ทดสอบพื้นที่ใกล้เต็ม | ยอดคงอยู่ ไม่มีรายการครึ่งชุด error อธิบายได้ |
| Backup/restore | backup → ลอง restore ลง test profile/เครื่องทดสอบ; รหัสผิด/ไฟล์ขาด/เวอร์ชันใหม่กว่า/พื้นที่เต็ม | success กลับครบ; failure รักษาฐานเดิม; ทดสอบรูปและกฎเตือนด้วย |
| PIN/biometric | ผิดซ้ำ ยกเลิก lockout ไม่มี biometric เปลี่ยน biometric, resume, ลืม PIN | ไม่มี bypass, fallback/recovery ตรง spec, ไม่ทำข้อมูลหาย |
| Privacy | airplane mode + สังเกต network ในโหมด online, logs, app-switcher snapshot, export temp files | feature หลักใช้ได้ offline, ไม่ส่งข้อมูลบัญชี, ไม่มี secrets ใน log/preview |
| กล้อง/รูป | อนุญาต/ปฏิเสธ/ถอนสิทธิ์ เลือกรูปเฉพาะบางรูป เปลี่ยนแอประหว่างถ่าย | ไม่มี crash มีกรอกมือเป็น fallback |
| Receipt OCR | corpus แยกจากที่ใช้จูน มีไทย/อังกฤษ ยับ ซีด เอียง เงา VAT เงินทอน ภาพสลิป | วัด total/date/merchant ราย field และบันทึกเวลา/peak RAM; review ก่อน commit |
| CSV | เปิดใน Excel/Numbers/Sheets; ไทย comma quote newline และข้อความขึ้นต้น = + - @ | ภาษาไม่แตก ยอดตรง escape ถูก ไม่มีสูตรอันตรายจากข้อความที่กรอก |
| PDF | ดูบน iPhone/desktop และหลายหน้า: สระ/วรรณยุกต์ไทย ตารางยาว ยอดท้าย | font shaping/embedded font ถูก ตารางไม่ทับ/ขาด ตัวเลขตรง DB |
| Reminders | app ปิด/foreground, จ่ายแล้ว/แก้วัน/ลบ, วันที่ 29–31, leap year, timezone, Focus/ปิดสิทธิ์ | ไม่ซ้ำ ยกเลิก occurrence ที่จ่ายแล้ว แจ้งข้อจำกัด OS/สิทธิ์ตามจริง |
| Insights | deterministic fixtures, เงินโอน/หนี้/พอร์ตขึ้น, ไม่มีรายรับ, ข้อมูลขาด | ไม่หารศูนย์ ไม่ปน valuation เป็นรายรับ ไม่ให้คะแนนเมื่อข้อมูลไม่พอ |
| Forecast | rolling-origin holdout ไม่ใช้ข้อมูลอนาคตเทรน; เทียบ weekday mean/simple baseline | รายงาน MAE และ coverage ตามจริง ก่อนอ้างว่าทำนายแม่น |
| ภาษี | coverage matrix/independent review, official fixtures, differential/boundary/cap/rounding/year-rule tests ตาม tax-engine-th.md | ยอดและสิทธิตรงกฎที่ตรวจแล้ว ข้อมูลไม่ครบ/กฎไม่พร้อมไม่แสดง finalized |
| VAT ต่างประเทศ | e-Service vs บริการอื่น, registration ตามวันที่, FX, VAT รวม/แยก, duplicate evidence และ due dates ตามแบบ | ไม่ตั้งหนี้จากชื่อ supplier หรือข้อความ reverse charge อย่างเดียว ไม่คิด/บันทึกซ้ำ |
| Payroll | gross/net/withholding/50 ทวิ/หลายนายจ้างและเงินรับจริง | ไม่ใช้ net deposit เป็น gross อัตโนมัติและไม่เครดิตภาษีหักไว้ซ้ำ |
| UI/accessibility | XCUITest/UI tests ตามความเหมาะสม + manual VoiceOver/TalkBack, text scaling, contrast, reduce motion, small screen | อ่านยอด/กดปุ่มได้ ไม่พึ่งสีอย่างเดียว keyboard ไม่บัง Save |
| ความเร็ว | release build + Android Profiler/เครื่องจริง, Windows profiling และ Instruments เมื่อพอร์ต iOS; 100 บัญชี/50,000 รายการ, สแกน/แชตต่อเนื่อง | UI ไม่ค้าง worker ยกเลิกได้ ไม่มี memory growth ต่อเนื่อง บันทึก cold/warm latency |

ผลผ่าน compile, Clippy หรือ unit tests ไม่ใช่หลักฐานว่า camera/biometric/signing/recovery ผ่าน ต้องแนบเครื่อง OS และ build number ของผล device tests

เสนอเกณฑ์เริ่มต้นให้เจรจาก่อน benchmark: corpus อย่างน้อย 200 ใบที่ได้รับอนุญาตและเก็บ local; total exact match ≥95% สำหรับภาพอ่านได้, แยกภาพยากรายงานอีกชุด; merchant/date วัดแยก; ทุกใบต้องยืนยันก่อนบันทึก ไม่อ้างผลนี้จนกว่าจะได้วัดจริง ค่า speed/memory budget ต้องเลือกหลังรู้รุ่น Android ต่ำสุดและทดสอบ release build แล้วประเมิน iPhone แยกเมื่อพอร์ต

สำหรับใบทดสอบ OCR ต้องมี ground truth ที่ตรวจด้วยคน เก็บ confidence แบบไม่อ้างว่าเป็นเปอร์เซ็นต์ความจริงโดยไม่ calibrate และตรวจความผิดพลาดยอดรวมแยกจาก character accuracy บนเอกสารทั่วไป

Android รอบแรกต้องทดสอบรุ่น RAM ต่ำสุดที่จะประกาศรองรับ, จอเล็ก, Android ต่ำสุด/ล่าสุด และอย่างน้อย OEM เป้าหมายที่พฤติกรรม background ต่างกัน Emulator ใช้ขยาย matrix แต่แทน hardware benchmarks ไม่ได้ เมื่อพอร์ต iOS จึงทดสอบ iPhone ของผู้ใช้/รุ่นเก่าสุดที่รองรับและ iOS ต่ำสุด/ล่าสุดแยก

**ขั้นตอนขึ้น Apple App Store — หลังรุ่น Android**

1. **พิสูจน์ release path ก่อน** — ทำแอปเล็กให้ลง iPhone และสร้าง archive/package ที่เซ็นถูก จากนั้นอัปโหลด build ทดสอบเข้า App Store Connect ตรวจ extensions, capabilities, native libraries และ model resources ว่าติดไปครบ การมีไฟล์ `.app` อย่างเดียวไม่ใช่จบการส่ง Store
2. **สมัครสมาชิก** — Apple Developer Program ราคาอ้างอิง 99 USD/ปี หรือสกุลเงินท้องถิ่นตามหน้าสมัคร ถ้าสมัคร Individual ชื่อผู้ขายเป็นชื่อบุคคลตามระบบของ Apple; Organization ต้องข้อมูลนิติบุคคลตามเกณฑ์ ดู [enrollment](https://developer.apple.com/help/account/membership/program-enrollment)
3. **สร้างรายการแอป** — กำหนดชื่อที่ว่าง Bundle ID, SKU, primary language, support contact, version และ build number ที่ไม่ซ้ำ
4. **เตรียมการขาย** — รับ Paid Apps Agreement กรอกธนาคารและ tax forms ที่ App Store Connect ขอ ตั้งประเทศไทยเป็น base region แล้วเลือก price point 349 บาทหากมีในตารางบัญชี ณ ตอนตั้งราคา หากไม่มีต้องเลือกราคาที่มีจริง ดู [setting price](https://developer.apple.com/help/app-store-connect/manage-app-pricing/set-a-price) และ [receiving payments](https://developer.apple.com/help/app-store-connect/getting-paid/overview-of-receiving-payments)
5. **ตรวจ SDK** — ข้อกำหนดที่ตรวจวันนี้: ตั้งแต่ 28 เมษายน 2026 ต้อง build ด้วย Xcode 26+ และ SDK iOS 26+; deployment target ต่ำกว่านั้นได้เท่าที่ dependencies/APIs รองรับ ไม่ได้หมายความว่าต้องให้ลูกค้าใช้ iOS 26 ทุกคน ดู [Apple requirements](https://developer.apple.com/news/upcoming-requirements/)
6. **อัปโหลด** — ใช้เส้นทาง archive/distribute ของ Xcode หรือเครื่องมือ upload ที่รองรับกับ package ที่เซ็นแล้ว รอ processing ตรวจ errors/symbols และทำ export compliance ตาม encryption ที่ใช้งานจริง ห้ามตอบ exemption โดยเดา ดู [export compliance](https://developer.apple.com/help/app-store-connect/manage-app-information/overview-of-export-compliance)
7. **TestFlight** — เริ่ม internal team; เปิด external beta ให้เพื่อน/ผู้ใช้จริงเมื่อ build พร้อม โดย external build แรกต้องผ่าน beta review ใช้ feedback และ crash diagnostics ที่สอดคล้อง privacy policy ทดสอบตั้งแต่ติดตั้งใหม่และอัปเกรดฐานเก่า ดู [TestFlight](https://developer.apple.com/testflight/)
8. **เตรียม Store listing** — app icon, screenshots จากแอปที่ทำงานจริงตามขนาดที่ Store ขอ, คำอธิบายไทย, keywords, support URL, privacy policy URL, age rating และคำตอบ privacy label รวม third-party SDKs ให้ครบ ภาพคอนเซปต์ AI รอบนี้ไม่ใช้แทน screenshot ฟีเจอร์ที่ทำจริง
9. **อธิบายให้ reviewer ทดลองได้** — ระบุว่าข้อมูลอยู่เครื่อง ไม่มี login backend, วิธีสร้างข้อมูลทดลอง, ทดสอบ OCR, ตั้งเตือน และเปิด PIN/biometric อย่าใส่บัญชีธนาคารหรือข้อมูลจริงของผู้ใช้
10. **Submit for Review** — เลือก build, ตรวจข้อมูลแอป, ส่ง review และตอบคำถาม/แก้ reject ตามจริง เลือก manual release ถ้าต้องการตรวจหน้าร้านก่อนเผยแพร่ ดู [submit app](https://developer.apple.com/help/app-store-connect/manage-submissions-to-app-review/submit-an-app/)
11. **หลังเปิดขาย** — ทดลองติดตั้ง build จาก Store และอัปเกรดจากเวอร์ชันก่อน เก็บ signing/release credentials อย่างปลอดภัย มี support และแก้ migration/ข้อมูลก่อนฟีเจอร์แต่งภาพ

TestFlight มีอายุ build 90 วัน จึงไม่ใช่ช่องใช้ beta รุ่นเดียวตลอดไป ตัวเลขปัจจุบันรองรับทีม internal 100 คนและ external ถึง 10,000 คน; ไม่มีข้อกำหนด Apple แบบเดียวกับ 12 คน/14 วันของ Google Play ดู [internal testers](https://developer.apple.com/help/app-store-connect/test-a-beta-version/add-internal-testers) และ [TestFlight](https://developer.apple.com/testflight/)

Privacy policy ต้องมีแม้ข้อมูลประมวลผล local รายการคำอนุญาตเช่นกล้อง/Face ID ต้องอธิบายตามการใช้จริง ขอเมื่อเริ่มใช้ฟีเจอร์ และปล่อยให้บันทึกมือได้เมื่อปฏิเสธ ดู [App Privacy Details](https://developer.apple.com/app-store/app-privacy-details/) และ [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)

**ราคา 349 บาทและค่าใช้จ่าย**

ข้อเสนอคือ paid download ซื้อครั้งเดียวผ่าน Store ใช้ฟีเจอร์ที่ขายได้ offline ไม่ต้องมีระบบ subscription/account server หากภายหลังเปลี่ยนเป็นโหลดฟรีแล้วซื้อ unlock ต้องทำ StoreKit / Play Billing และทดสอบ restore purchase เพิ่ม

Apple Small Business Program มี commission 15% สำหรับผู้ที่สมัครและมีสิทธิ์ตามเงื่อนไข ไม่ใช่ทุกบัญชีได้รับอัตโนมัติ ดู [Small Business Program](https://developer.apple.com/app-store/small-business-program/)

349 × 0.85 = 296.65 บาท เป็นเพียงตัวอย่างหัก commission อย่างเดียว; ยอดรับจริงขึ้นกับฐานหลังภาษีตาม Store, adjustments และเงื่อนไขบัญชี ใช้ proceeds ใน App Store Connect วางแผนรายรับ อย่าถือ 296.65 เป็นเงินสุทธิจริง ดู [pricing/proceeds](https://developer.apple.com/help/app-store-connect/reference/pricing-and-availability/app-pricing-and-availability)

ต้นทุน recurring แม้ไม่มี inference server ได้แก่สมาชิก Apple, เครื่อง/เช่า build, support, การตาม iOS/Android และ library/model updates ราคา 349 เป็นสมมติฐานผลิตภัณฑ์ ต้องทดสอบ willingness to pay กับ beta users ไม่มีข้อมูลในงานนี้ที่พิสูจน์ว่าจะขายได้กี่ชุด

**ขั้นตอนขึ้น Android / Google Play — Store แรก**

1. ติดตั้ง Android Studio, SDK/NDK/JDK ตามเวอร์ชันที่ pin และพิสูจน์ Dioxus + Rust/native OCR libraries บน arm64 อุปกรณ์จริง
2. เชื่อม camera/photo picker, BiometricPrompt, Keystore, local reminders, share/export และ optional NotificationListenerService แยก Android adapter
3. ตรวจ native library compatibility กับเครื่อง 16 KB page size เพราะมี Rust/SQLite/OCR native binaries ดู [Android page-size guide](https://developer.android.com/guide/practices/page-sizes)
4. สร้าง signed Android App Bundle `.aab` และตั้ง Play App Signing; output `.apk` สำหรับติดตั้งทดสอบไม่ใช่ release artifact ที่จะสมมติว่าใช้แทน `.aab` ได้ ต้องพิสูจน์ Gradle/packaging path ของ Dioxus เวอร์ชันที่เลือก
5. สมัคร Play Console มีค่าลงทะเบียน 25 USD ครั้งเดียว พร้อม verification และ merchant setup ที่บัญชีต้องทำ ดู [Play Console registration](https://support.google.com/googleplay/android-developer/answer/6112435?hl=en)
6. กรอก Data safety, privacy policy, content rating, financial-features declaration ตามฟีเจอร์จริง, app access, ประเทศและราคา และข้อมูลหน้าร้าน
7. ข้อกำหนดที่ตรวจวันนี้: แอปมือถือใหม่/อัปเดต target Android 16 / API 36+ ตั้งแต่ 31 สิงหาคม 2026 โดย minSdk เป็นคนละค่า ดู [target API requirements](https://support.google.com/googleplay/android-developer/answer/11926878?hl=en)
8. บัญชี personal ใหม่ที่อยู่ในเกณฑ์หลัง 13 พฤศจิกายน 2023 ต้อง closed test อย่างน้อย 12 testers ที่ opted-in ต่อเนื่อง 14 วัน จากนั้นสมัคร production access และให้ข้อมูลการทดสอบ การครบจำนวนวันไม่ใช่การอนุมัติอัตโนมัติ ดู [Google testing requirements](https://support.google.com/googleplay/android-developer/answer/14151465?hl=en)
9. ทำ internal/closed test, pre-launch report, ตรวจ Pixel และอย่างน้อยอีก OEM ที่จะรองรับ โดยเฉพาะ battery restrictions, notification access revoked, reboot และ format notification ของธนาคาร
10. ส่ง production review เมื่อ checklists ผ่าน แล้วติดตาม crashes/ANRs และการอัปเกรดข้อมูลตาม privacy policy ที่แจ้ง

ซื้อแอปบน Apple ไม่ได้ทำให้ Google Play รู้ว่าเป็นผู้ซื้อรายเดียวกันโดยอัตโนมัติ ข้อเสนอเริ่มต้นคือซื้อแยกตาม Store; หากต้องการ license ข้าม Store ต้องเพิ่มวิธีพิสูจน์สิทธิ์และนโยบายที่ไม่ขัด offline/privacy promise

**หลักฐานที่เก็บก่อนส่งแต่ละเวอร์ชัน**

บันทึก app version/build, commit, Rust/Dioxus/SDK/runtime/model versions, model checksum/license, device/OS, ผล core/storage/device tests, OCR corpus metrics, backup restore evidence, known issues และ package upload result ไว้ใน release checklist ไม่ใส่ข้อมูลบัญชี/ใบเสร็จจริงใน public repo

ข้อหยุด release: ยอดคลาดแม้หนึ่งสตางค์ในกรณีที่รองรับ, duplicate commit, migration/restore ทำข้อมูลหาย, lock bypass, backup ใช้ไม่ได้, permission denial ทำแอปพัง หรือข้อความ privacy ไม่ตรงพฤติกรรมจริง

# ประวัติทิศทางภาพ v2 — soft mint / sage

ทิศทางนี้ถูกแทนที่แล้ว ดู [ธีมปัจจุบัน](pastel-companion-theme.md) ภาพคอนเซปต์ PNG เก่าถูกนำออกจากโปรเจกต์ระหว่างจัด repository

ผู้ใช้เลือก layout จากภาพ profile UI ใหม่ (ภาพที่ 2) และ palette จากภาพถนนฝนตก (ภาพที่ 1) ทิศทางนี้แทน comic/pink/halftone ของ v1

คอนเซปต์ที่เคยเลือก: Desktop + Android quick-entry (`ledgers-bro-ui-concept-v2-final.png`) สร้างด้วย built-in imagegen โดยใช้ภาพทั้งสองตามบทบาทที่ผู้ใช้ระบุ และใช้ mascot6.png ของผู้ใช้เป็น reference ตัวละคร ภาพเป็น bitmap สำหรับทิศทาง ยังไม่ใช่ Dioxus UI ที่กดได้

Prompt ต้นฉบับและคำสั่งแก้ไอคอนอยู่ที่ [concept-prompt-v2.txt](concept-prompt-v2.txt) รุ่น final เปลี่ยน cloud เป็น device icon ตรงข้อความข้อมูลในเครื่อง ภาพทั้งรุ่นก่อนแก้และ final ไม่ได้เก็บใน repository แล้ว

**องค์ประกอบที่ต้องรักษา**

- การ์ดมุมมน ระยะหายใจมาก เส้นบาง เงานุ่ม pill navigation และตัวเลขสี slate เข้มอ่านง่าย
- ตัวละครของผู้ใช้ผมดำแว่นกลม เสื้อดำรายละเอียด plaid และกีตาร์ ทำหน้าที่เป็นเพื่อนในหน้าภาพรวม desktop มีพื้นที่ art มากกว่า mobile
- สีเขียว seafoam/mint, sage/eucalyptus, cream, slate และ dusty peach เป็น accent; ไม่เอาสีชมพูจากภาพ layout มาเป็นสีหลัก
- รูปถนนใช้เพื่อบอกสีและบรรยากาศ ไม่ใช้ตัวละคร/ข้อความ/ตราของ reference เป็นตัวตนแอป
- หน้า mobile เน้นบันทึกและ choices; ไม่ย่อ desktop ทั้งหน้าให้ตัวอักษรเล็ก ใช้ responsive composition ตามขนาดและ input method

Palette ตั้งต้นสำหรับ implementation: canvas #EAF1E9, card #FAFBF6, mint #C5E1D8, sage #A8B9A8, teal #4F8986, slate #243B46, peach #DDB5A1 ต้องตรวจ contrast ของ actual foreground/background ก่อนใช้ ไม่ใช่ชุดสีที่ audit แล้ว

**หน้าจอในภาพ**

Desktop: top nav ภาพรวม/บัญชี/รายการ/แชต/ภาษี, net worth card, income/expense, expense donut, รายการล่าสุด, tax completeness card และช่อง quick entry พร้อมปุ่มตัวอย่าง

Android chat: user พิมพ์ “กาแฟ 80” → draft ยอด 80 บาท → choices บัญชีและหมวด → ข้อความยังไม่ได้บันทึก → ปุ่มตรวจรายการยัง disabled จนข้อมูลพร้อม → จากนั้นจึงแสดง confirmation card และให้ผู้ใช้กด Save ใน implementation

Tax card ใน mockup เป็น “ข้อมูลยังไม่ครบ” ไม่มีเลขที่ดูเหมือนยอดภาษีตามกฎหมายจริง ภาพ/ยอด/เปอร์เซ็นต์ทั้งหมดเป็น sample data ไม่ใช่ fixture ผลคำนวณบัญชี

**รายละเอียดสำหรับ UI จริง**

ใช้ RSX/CSS และ icon assets แยก component ไม่วาง screenshot เป็นพื้นแล้วทำ hotspot; ตัวอักษรไทยและตัวเลขต้องเป็นข้อความที่ screen reader อ่านได้, keyboard navigation บน desktop และ touch targets บน Android ต้องทดสอบ

ใช้ device/storage icon สำหรับข้อมูลในเครื่อง; ไม่ใช้ cloud icon เพราะสื่อผิดเรื่อง sync หน้าภาษีแสดงปี rule version/วันตรวจในรายละเอียดที่เปิดดูได้ ไม่ยัด implementation metadata ลงหน้าหลัก

ลดของตกแต่ง/ฉากหลังจากคอนเซปต์ในหน้ากรอกและหน้าตัวเลขหนาแน่น ให้ผู้ใช้เปิดโหมดเรียบได้ในอนาคต ตัวละครไม่บังยอด ปุ่ม หรือ scroll area ไม่ทำภาพแอนิเมชันจำเป็นต่อการอ่านข้อมูล

สถานะเลือกบัญชี/หมวดต้องมี border/เครื่องหมาย/ข้อความร่วมกับสี, focus มองเห็น, large text/VoiceOver/TalkBack/reduce motion ต้องผ่าน ส่วน disabled button ต้องมีข้อความอธิบายข้อมูลที่ยังขาด

ในภาพยังมีความเป็น concept เช่น window chrome ผสมลักษณะแพลตฟอร์ม ตัวอักษรบางส่วนและภาพตกแต่งที่ generated เพิ่ม เมื่อ implement ใช้ native title bar ของ Windows/Android safe areas จริง และตัดสินใจรายละเอียด art ด้วยผู้ใช้ แทนการคัด bitmap ทุกพิกเซล

Store screenshots ต้องจับจาก build จริงหลังฟีเจอร์ทำงาน จัด editable original art ของผู้ใช้และ exports แยก role เพื่อเปลี่ยนภาพ AI เป็นภาพวาดจริงได้โดยไม่แก้ core

# สร้างและติดตั้ง Windows MSI

รันจาก root ของโปรเจกต์บน Windows x64 ใช้ Python 3.10+, Rust MSVC,
Visual Studio C++ Build Tools, CMake, libclang และ Dioxus CLI 0.7.2:

```powershell
cd D:\SideProjects\ledgers-bro
cargo install dioxus-cli --version 0.7.2 --locked
python scripts/build-windows.py --check
python scripts/build-windows.py
```

ถ้าติดตั้ง Dioxus รุ่นนี้ไว้แล้ว ไม่ต้องติดตั้งซ้ำ ใช้ `py` แทน `python` ได้หากเครื่องใช้ Python Launcher
สคริปต์หา libclang จาก LLVM หรือ Android SDK/NDK ในเครื่อง หากอยู่ที่อื่นให้ตั้ง
`$env:LIBCLANG_PATH` เป็นโฟลเดอร์ที่มี `libclang.dll` ก่อนรัน

ต้องมีชุด OCR ใน `.tools/ocr` ตาม [คู่มือ OCR](receipt-ocr.md) หรือส่ง
`--ocr-dir "D:\path\to\ocr"` ชุดนี้ต้องมี Tesseract พร้อม DLL, โมเดล Thai/English
และ ImageMagick portable ใน `image-decoder/` สคริปต์ตรวจ checksum ของโมเดลภาษา
และลองเรียก native runtime ก่อนเริ่ม build

สคริปต์เตรียมไฟล์ใน `target/windows-resources/` แล้วเรียก:

```powershell
cd apps/desktop
dx bundle --desktop --release --package ledgers-bro --package-types msi --locked --out-dir ../../target/installers
```

ให้ใช้สคริปต์เป็นหลักเพื่อไม่ลืมเตรียม OCR และ licenses หลัง `cargo clean`
ครั้งแรก Dioxus อาจดาวน์โหลดเครื่องมือ WiX และ WebView2 bootstrapper จึงต้องมีอินเทอร์เน็ต
ดู [Dioxus bundling](https://dioxuslabs.com/learn/0.7/tutorial/bundle/)

## ติดตั้งและเปิดใช้งาน

1. เปิด `target/installers/` แล้วดับเบิลคลิกไฟล์ `.msi`
2. ทำตามหน้าติดตั้ง แล้วเปิด **Ledgers Bro** จาก Start Menu
3. การติดตั้งนี้มีตัวอ่านใบเสร็จและ HEIC/HEIF decoder ส่วน AI ให้กด
   **ดาวน์โหลด AI ในเครื่อง** ในแอปครั้งแรก ประมาณ 397 MB หลังจากนั้นใช้ได้ออฟไลน์
4. หากจะถอนการติดตั้ง ใช้ Windows Settings → Apps → Installed apps → Ledgers Bro

ตัวติดตั้งที่ build เองยังไม่ได้ลงนามด้วยใบรับรอง code signing จึงอาจแสดงผู้เผยแพร่เป็น Unknown publisher
นี่เป็น build สำหรับทดลองในเครื่อง ยังไม่ใช่การรับรองการเผยแพร่บน Store

ข้อมูลผู้ใช้เก็บใน local application data ของ Windows โดยค่าเริ่มต้นเป็น
`%LOCALAPPDATA%\Dancing With My Code\Ledgers Bro\data\ledger.sqlite3`
แยกจาก `.data/sandbox` ที่ใช้ทดลองผ่าน Cargo ตัวติดตั้งไม่รวมข้อมูลบัญชีของผู้พัฒนา
และไม่คัดลอก sandbox ให้อัตโนมัติ ให้ปิดแอปและสำรองฐานข้อมูลก่อนเปลี่ยน build

## ขอบเขตไฟล์ที่แพ็ก

- Release executable, ไอคอน Windows จาก artwork เดิม และ assets ที่ฝังในแอป
- Tesseract executable/DLL, Thai/English tessdata และ notices
- ImageMagick portable directory พร้อม licenses เดิม
- ไม่รวม `.data/`, ledger/export ส่วนตัว หรือโมเดล LLM ที่ดาวน์โหลดไว้ในเครื่องผู้พัฒนา

ชื่อผลิตภัณฑ์และ `upgrade_code` ใน `apps/desktop/Dioxus.toml` ต้องคงที่เมื่ออัปเดต
และต้องเพิ่ม version ก่อนแจก release ถัดไป เพื่อให้ Windows จัดการ upgrade ได้ถูกตัว

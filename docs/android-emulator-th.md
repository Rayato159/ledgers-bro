# ทดลอง Android APK บน Windows

สถานะ 21 กันยายน 2026: มี Android host ใน `apps/android` และ build APK รุ่นทดลองได้ทั้ง x86_64 สำหรับ Emulator กับ ARM64 สำหรับมือถือแล้ว ใช้ domain/application/SQLite/UI ชุดเดียวกับ Windows แยกแพ็กเกจเป็น `com.dancingwithmycode.ledgersbro.test` ชื่อบนเครื่อง **Ledgers Bro Test** ตรวจเปิดแอปจริงบน Emulator แล้ว ส่วน ARM64 ยังไม่ได้ทดสอบบนมือถือจริง

## เปิดแอปบน emulator ที่รันอยู่

เปิด Android Studio → **Device Manager → กด ▶ ที่ Pixel_6** แล้วรอเข้า Android จากนั้นเปิดไอคอน **Ledgers Bro Test** ได้เลย หากจะติดตั้ง APK ล่าสุดทับ ให้เปิด PowerShell:

```powershell
cd D:\SideProjects\ledgers-bro
.\scripts\run-android.ps1 -Device emulator-5554
```

ไฟล์อยู่ที่ `target/android/ledgers-bro-test-x86_64.apk` สคริปต์ตรวจว่า device ออนไลน์และรองรับ ABI ก่อนติดตั้งทับด้วย `adb install -r` แล้วเปิดแอป ไม่ถอนแอปหรือล้างข้อมูล หากมีเครื่องเดียวออนไลน์สามารถละ `-Device` ได้ หาก serial ต่างจากตัวอย่างให้ใช้ค่าจาก `adb devices` ตามหัวข้อ 6

หลังแก้โค้ด ให้ build ก่อนติดตั้งใหม่:

```powershell
.\scripts\build-android.ps1
.\scripts\run-android.ps1 -Device emulator-5554
```

ไม่ต้อง `cargo clean` ทุกครั้ง เพราะทำให้คอมไพล์ใหม่ทั้งหมด `cargo clean` ลบผล build ใน `target` รวมถึง APK แต่ไม่ลบข้อมูลแอปใน Emulator/มือถือ และไม่ลบ `.data/playground` ของ Windows

Build ใช้ Dioxus CLI 0.7.2 และ Cargo.lock ตั้ง environment เฉพาะตอนทำงานและคืนค่าเมื่อจบ รับ `-SdkRoot`, `-NdkRoot`, `-JdkRoot` ได้ เลือก JDK 21 ที่ติดตั้งอยู่ก่อน JBR ใช้ Java temporary directory ใน `.tools/java-tmp` เพื่อแก้ Windows AF_UNIX ที่ล้มเหลวกับ TEMP แบบ 8.3 และไม่ปล่อย Gradle daemon ค้างหลัง build

แต่ละ APK กรอง native libraries ให้ตรง ABI ที่เลือก และตรวจไฟล์ก่อนคัดลอกไป `target/android` พร้อมไฟล์ SHA-256 ป้องกันไลบรารีจากการ build target ก่อนหน้าปะปนกัน Dioxus ใช้ staging directory ร่วมกัน จึงให้ build สอง target **ทีละตัว ไม่เปิด build พร้อมกัน**

CLI เวอร์ชันนี้สร้าง Gradle 9.1.0 / AGP 8.7.0 / Kotlin 2.0.20 / compileSdk และ targetSdk 33; minSdk ของ host คือ 24 Gradle อาจดาวน์โหลด SDK 33 และ Build Tools เพิ่มหลังผู้ใช้ยอมรับ license ใน SDK Manager แล้ว **ยังไม่ใช่ชุดเครื่องมือและ target SDK สำหรับส่ง Store** ดู [ผลตรวจ Android](verification-android-2026-09-21.md)

## 1. ติดตั้งเครื่องมือ

ติดตั้ง [Android Studio](https://developer.android.com/studio) แล้วเปิด SDK Manager เลือก Android SDK Platform, Platform-Tools, Android Emulator, Command-line Tools (latest), NDK (Side by side) และ CMake ตาม [Dioxus 0.7 mobile setup](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/). SDK/NDK/system image ใช้พื้นที่หลาย GB; อ่านและยอมรับ license ในหน้าติดตั้งด้วยตัวเอง

สคริปต์ build ด้านบนตั้ง environment ให้แล้ว หากต้องตั้งเอง ใช้ JDK 21 ที่ตรวจ build ผ่านบนเครื่องนี้ และแทนเวอร์ชัน NDK ให้ตรงโฟลเดอร์ที่ติดตั้งจริง (JBR ที่มากับ Android Studio บนเครื่องนี้เป็น Java 25 ยังไม่ได้ตรวจ build):

```powershell
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-21'
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
Get-ChildItem "$env:ANDROID_HOME\ndk" -Directory
# แทน <เวอร์ชันที่ติดตั้ง> ก่อนรันบรรทัดถัดไป
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\<เวอร์ชันที่ติดตั้ง>"
$env:PATH = "$env:JAVA_HOME\bin;$env:ANDROID_HOME\platform-tools;$env:ANDROID_HOME\emulator;$env:PATH"
rustup target add x86_64-linux-android aarch64-linux-android
```

`x86_64-linux-android` สำหรับ emulator x86_64 บนคอมนี้; `aarch64-linux-android` สำหรับโทรศัพท์ ARM64/AVD ที่ใช้สถาปัตยกรรมนั้น ไม่ถือว่า APK คนละ ABI ใช้แทนกันได้ทุกเครื่อง

ตรวจสภาพแวดล้อมโดยไม่ติดตั้งหรือเปลี่ยนระบบ:

```powershell
cd D:\SideProjects\ledgers-bro
.\scripts\check-android.ps1
```

สคริปต์รับ `-SdkRoot`, `-JdkRoot`, `-NdkRoot` หากติดตั้งนอกตำแหน่งปกติ

## 2. สร้างเครื่องจำลอง

เปิด **Device Manager → Create Device → โทรศัพท์ Pixel → system image x86_64 → Finish → Run** เครื่องที่ตรวจในงานนี้คือ Pixel 6 / Android 17 API 37 / x86_64; ไม่ใช่ผลยืนยัน Android ทุกเวอร์ชัน วิธีสร้าง AVD: [Android Developers](https://developer.android.com/studio/run/managing-avds)

เปิด virtualization ใน BIOS/UEFI และ Windows Hypervisor Platform ถ้ายังปิดอยู่; อาจต้อง reboot ดู [hardware acceleration](https://developer.android.com/studio/run/emulator-acceleration). งานนี้ไม่ได้เปลี่ยน Windows features หรือรีบูตเครื่องให้

```powershell
emulator -list-avds
adb devices
```

ต้องเห็น serial เช่น `emulator-5554` มีสถานะ `device`; ถ้ามีหลายเครื่อง ระบุ `adb -s <serial>` ทุกครั้ง

## 3. ส่วนที่พอร์ตแล้วและงานที่ยังเหลือ

| ส่วน | ปัจจุบัน | Android ที่ต้องเพิ่ม |
|---|---|---|
| Domain / application / UI | ใช้ร่วมกันและ build/เปิดบน emulator x86_64 ผ่าน | ตรวจ lifecycle/IME และอุปกรณ์จริงเพิ่ม |
| Composition root | Windows ใช้ `apps/desktop` | เพิ่ม `apps/android` พร้อม manifest และ Kotlin activity แยกแล้ว |
| SQLite | Windows ProjectDirs / data-dir | ใช้ `Context.getNoBackupFilesDir()` ใน sandbox แอป ไม่เข้า Auto Backup; ยังไม่เข้ารหัส |
| Export CSV | Native Windows save dialog | เพิ่ม `ACTION_CREATE_DOCUMENT` และเขียน UTF-8 ผ่าน ContentResolver บน worker thread พร้อมผลสำเร็จ/ยกเลิก/ผิดพลาด; lifecycle ระหว่าง export ยังต้องตรวจเพิ่ม |
| OCR | Windows: Tesseract + ตัวแปลง HEIF | Android 9+: native document picker, ImageDecoder และ Tesseract4Android พร้อม Thai/English models ใน APK; รองรับ JPG/PNG/HEIC/HEIF ต้องทดสอบความแม่นกับร้านและเครื่องจริงเพิ่ม |
| Art / font | ฝังใน desktop executable | ฝัง assets และ TH Sarabun New ใน Android แล้ว; ยังต้องวัด memory/ขนาดสำหรับ release |
| Backup | ยังไม่มีระบบสำรองที่กู้คืนได้ | ปิด backup ใน manifest และเก็บ DB ใน no-backup directory; ถอนแอปหรือล้าง app data ทำให้ข้อมูลหาย |

ใช้สคริปต์ build ด้านบนกับ `apps/android` ซึ่งรวม native OCR และโมเดลภาษาให้อัตโนมัติผ่าน Gradle home ของโครงการ ปัจจุบัน local LLM ทำงานบนเครื่องแล้ว ส่วน PIN/biometric, encrypted backup, PDF, ภาษีคำนวณจริง และ bank notification import ยังไม่ได้ทำ ดู [การนำรูปจาก iPhone มาเข้า Emulator](iphone-images-th.md)

แก้ Kotlin/manifest ที่ `apps/android/native` แล้ว build ใหม่ ไม่แก้ไฟล์ที่สร้างใน `target/dx` โดยตรง Dioxus ใช้ authenticated WebSocket บน `127.0.0.1` เพื่ออัปเดต UI จึงต้องมี INTERNET permission และ network-security config สำหรับ loopback แอปไม่มี backend สมุดบัญชีหรือ Cloud sync หากผู้ใช้เลือก provider บน Cloud ในหน้าต่าง Export ไฟล์จะถูกบันทึกตามตำแหน่งที่เลือก

## 4. ติดตั้ง APK ที่สร้างสำเร็จ

ลาก APK ลงหน้าจอ emulator หรือใช้:

```powershell
adb -s emulator-5554 install -r 'พาธจริงของไฟล์.apk'
```

`-r` อัปเดตแพ็กเกจเดิมและเก็บข้อมูลเมื่อ package/signing เข้ากันได้ อย่าแก้ปัญหา signature ด้วยการถอนแอปหรือ Wipe Data ถ้ามีข้อมูลที่ต้องเก็บ ดู [การติดตั้ง APK บน emulator](https://developer.android.com/studio/run/emulator-install-add-files)

## 5. ตรวจบน emulator ก่อนนำไปใช้

บันทึกด้วยเสียง: **บันทึกด่วน → พูดรายการ** ต้องมีบริการถอดเสียงภาษาไทยออฟไลน์ของ Android 12+; emulator แต่ละ image อาจไม่รองรับ แม้มี Google speech service แล้วก็ตาม แอปจะแสดงเหตุผลและยังพิมพ์ต่อได้ ดู [วิธีใช้เสียงและรายการทดสอบ](voice-entry-th.md)

- เริ่มแบบ airplane mode: สร้างบัญชี บันทึก/โอน/ยกเลิก/ลบ และตรวจยอดหลังปิดเปิดใหม่
- หมุนจอ เปลี่ยนแอป/กลับมา เปิดคีย์บอร์ดไทย กด Back ระหว่างฟอร์มและหน้าต่างยืนยัน
- ปิดแอประหว่างบันทึกด้วยข้อมูลทดสอบ แล้วเปิดใหม่ตรวจว่าไม่มีรายการซ้ำหรือครึ่งรายการ
- ส่งออกทั้งไทย/อังกฤษผ่าน Android document picker ไป Downloads เปิด CSV ตรวจสระ/วรรณยุกต์ bullets และเดบิตเท่ากับเครดิต; ทดสอบกดยกเลิกด้วย
- OCR ต้องทดสอบ adapter จริงพร้อมภาพใบเสร็จไทย อ่านไม่ได้ต้องกลับไปแก้มือได้
- อัปเกรด APK ทับเวอร์ชันก่อน ตรวจฐานข้อมูลและ signing; เครื่องมือทดสอบต้องใช้ package/data แยกจากข้อมูลจริง

ก่อนขายยังต้องทดสอบ Android เครื่องจริง โดยเฉพาะกล้อง ไฟล์ PIN/biometric notification หน่วยความจำ ความร้อน และประหยัดแบต Emulator อย่างเดียวไม่ยืนยันเรื่องเหล่านี้

## 6. เอาลงมือถือ Android จริงผ่าน USB

เส้นทางนี้ไม่ต้องเผยแพร่บน Play Store ใช้ APK `target/android/ledgers-bro-test-arm64.apk` ที่เตรียมไว้แล้วได้เลย หลังแก้โค้ดค่อย build target มือถือใหม่:

```powershell
.\scripts\build-android.ps1 -Target aarch64-linux-android
.\scripts\run-android.ps1 -Device 'SERIAL_ของมือถือ' -Abi arm64
```

ไฟล์ออกเป็น `target/android/ledgers-bro-test-arm64.apk` และยังเป็น debug signature **ผลตรวจ emulator ไม่ยืนยันว่าทดสอบ ARM64 หรือมือถือจริงแล้ว**

1. เตรียม SDK/NDK ตามหัวข้อ 1; ถ้าทดสอบเฉพาะมือถือจริง ยังไม่จำเป็นต้องดาวน์โหลด emulator/system image ใช้ Rust target ให้ตรง ABI ของมือถือ (ARM64 ใช้ `aarch64-linux-android`)
2. บนมือถือเข้า Settings → About phone → Build number แล้วแตะ 7 ครั้งเพื่อเปิด Developer options ตำแหน่งอาจอยู่ใน Software information ตามยี่ห้อ จากนั้นเปิด **USB debugging** ดู [การเปิด Developer options](https://developer.android.com/studio/debug/dev-options)
3. ต่อสาย USB ที่ส่งข้อมูลได้ ปลดล็อกมือถือและกดยอมรับการ debug จากคอมนี้ ถ้า Windows ไม่พบอุปกรณ์ ให้ตรวจสายและไดรเวอร์ ADB ของผู้ผลิตตาม [คู่มือทดสอบเครื่องจริง](https://developer.android.com/studio/run/device)
4. ใน PowerShell ใช้ ADB จาก SDK ที่ติดตั้งจริง ตัวอย่างนี้ใช้ตำแหน่งติดตั้งมาตรฐาน:

```powershell
$androidAdb = Join-Path $env:LOCALAPPDATA 'Android\Sdk\platform-tools\adb.exe'
& $androidAdb devices -l
```

ต้องเห็น serial ของมือถือและสถานะ `device`; ถ้า `unauthorized` ให้ปลดล็อกและยอมรับข้อความบนมือถือ ถ้าไม่มีรายการ ให้แก้การเชื่อมต่อก่อน

5. จากโฟลเดอร์โปรเจกต์ แทน `PHONE_SERIAL` ด้วย serial จากคำสั่งข้างบน สคริปต์ตรวจว่ามือถือรองรับ ARM64 แล้วติดตั้งและเปิดแอป:

```powershell
cd D:\SideProjects\ledgers-bro
.\scripts\run-android.ps1 -Device 'PHONE_SERIAL' -Abi arm64
```

ถ้าขึ้น `Success` ให้เปิดไอคอนแอปบนมือถือ; `-r` ใช้อัปเดตแอปเดิมโดยเก็บข้อมูลเมื่อ package/signing เข้ากันได้ การติดตั้งทับที่ล้มเหลวเพราะลายเซ็นไม่ตรงต้องแก้ build/signing ไม่ถอนแอปที่มีข้อมูลสำคัญ ดู [ADB: install an app](https://developer.android.com/tools/adb#move)

รอบแรกใช้ข้อมูลทดสอบ: สร้างบัญชี บันทึกรายรับ/จ่าย/โอน ปิดเปิดแอปและรีสตาร์ตมือถือเพื่อตรวจข้อมูลคงอยู่ เปิดโหมดเครื่องบิน ลองคีย์บอร์ดไทยและปุ่ม Back แล้วตรวจ export ไทย/เดบิตเครดิต ส่วน OCR ให้ลอง JPG/HEIC จากเครื่องจริง เทียบทุกบรรทัดและยอดสุทธิ รวมถึงเลือกรูปซ้ำ ยกเลิก และภาพเสีย การผ่านบน Emulator ไม่ยืนยันกล้องหรือ codec ของมือถือทุกรุ่น

## 7. เปิดเป็นแอป Windows โดยตรง

```powershell
cd D:\SideProjects\ledgers-bro
.\scripts\run-desktop.ps1
```

สคริปต์ build และเปิดแอปให้ โดยใช้ฐานข้อมูลทดสอบ `.data/playground` แยกจาก Android หลัง `cargo clean` ครั้งแรกจะคอมไพล์ใหม่ ไม่ต้องเปิด Android Studio สำหรับทางนี้

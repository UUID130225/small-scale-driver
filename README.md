# Precision Scale Driver (Rust) / Trình điều khiển Cân Điện Tử

[English Version Below](#english-version)

Một bộ driver RS232 mạnh mẽ và hiệu quả dành cho các dòng cân điện tử chính xác (như Hucle, XK3190...), được viết bằng ngôn ngữ Rust. Driver có tính năng tự động dò tìm thông số, lưu cấu hình và tùy chỉnh hệ số nhân.

## Tính năng

- **Tự động tìm cổng:** Quét và liệt kê các cổng COM/TTY khả dụng.
- **Dò tìm thông số thông minh:** Tự động thử các mức Baud Rate, Parity, Data Bits, và Stop Bits để tìm cấu hình đúng của cân.
- **Quét nhanh (Fast Scan):** Tối ưu hóa thời gian chờ (300-500ms) để nhận diện phần cứng nhanh chóng.
- **Lưu cấu hình:** Lưu các thông số thành công vào file `config.json` để kết nối ngay lập tức trong các lần chạy sau.
- **Trích xuất dữ liệu bằng Regex:** Tự động tách số cân nặng từ các chuỗi dữ liệu phức tạp (ví dụ: `+00.17150TL S`).
- **Hệ số nhân (Multiplier):** Tùy chỉnh hệ số nhân để khớp với màn hình hiển thị của cân khi dữ liệu gửi qua RS232 bị lệch (ví dụ: gửi 0.17 nhưng cân hiện 1.7).

## Cài đặt

1.  **Cài đặt Rust:** Nếu bạn chưa có Rust, hãy tải tại [rustup.rs](https://rustup.rs/).
2.  **Tải mã nguồn:**
    ```bash
    git clone https://github.com/UUID130225/small-scale-driver.git
    cd small-scale-driver
    ```
3.  **Biên dịch dự án:**
    ```bash
    cargo build --release
    ```

## Cách sử dụng

### Lần chạy đầu tiên (Thiết lập)
Chạy driver mà không cần tham số. Chương trình sẽ yêu cầu bạn chọn cổng và sau đó tự động dò tìm thông số.
```bash
cargo run
```

### Sử dụng hệ số nhân
Nếu cân hiện `1.7` nhưng driver hiện `0.17`, hãy sử dụng tham số multiplier:
```bash
cargo run -- --multiplier 10
```

### Chỉnh sửa cấu hình thủ công
Bạn có thể tự sửa file `config.json` được tạo ra sau lần chạy thành công đầu tiên:
```json
{
  "port_name": "COM3",
  "baud_rate": 1200,
  "data_bits": "Eight",
  "parity": "None",
  "stop_bits": "Two",
  "multiplier": 10.0
}
```

### Ép buộc dò tìm lại
Nếu bạn đổi cân hoặc đổi cổng kết nối:
```bash
cargo run -- --force-detect
```

---

<a name="english-version"></a>
# English Version

A robust and efficient RS232 driver for precision scales (e.g., Hucle, XK3190, etc.) written in Rust. It features automatic parameter detection, configuration persistence, and customizable scale factors.

## Features

- **Port Discovery:** Automatically scans and lists available COM/TTY ports.
- **Smart Auto-Detection:** Brute-forces Baud Rate, Parity, Data Bits, and Stop Bits to find the correct settings for your scale.
- **Fast Scanning:** Optimized timeout (300-500ms) for quick hardware discovery.
- **Config Persistence:** Saves successful settings to `config.json` for instant connection on subsequent runs.
- **Regex Extraction:** Automatically extracts numeric weight values from complex serial strings (e.g., `+00.17150TL S`).
- **Scale Factor (Multiplier):** Adjustable multiplier to match the scale's visual display when the serial output is scaled differently.

## Installation

1.  **Install Rust:** If you don't have Rust installed, get it at [rustup.rs](https://rustup.rs/).
2.  **Clone the repository:**
    ```bash
    git clone https://github.com/UUID130225/small-scale-driver.git
    cd small-scale-driver
    ```
3.  **Build the project:**
    ```bash
    cargo build --release
    ```

## Usage

### First Run (Setup)
Run the driver without arguments. It will ask you to select a port and then attempt to auto-detect the scale settings.
```bash
cargo run
```

### With Scale Factor
If your scale displays `1.7` but the driver shows `0.17`, use the multiplier:
```bash
cargo run -- --multiplier 10
```

### Manual Configuration
You can manually edit the `config.json` file:
```json
{
  "port_name": "COM3",
  "baud_rate": 1200,
  "data_bits": "Eight",
  "parity": "None",
  "stop_bits": "Two",
  "multiplier": 10.0
}
```

## License

MIT

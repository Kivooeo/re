
# Используем официальный образ Rust
FROM rust:latest

# Устанавливаем рабочую директорию
WORKDIR /app

# Копируем файлы проекта в контейнер
COPY . .

# Сборка приложения
RUN cargo build --release

# Указываем исполняемый файл для запуска
CMD ["./target/release/re"]

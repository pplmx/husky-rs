# Stage 1: Build the Rust project using the official Rust image
FROM rust:1-bookworm as builder

# Install required dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /app

# Copy the project files into the container
COPY . .

# Build the project in release mode
RUN cargo build --release

# Stage 2: Create a minimal image using distroless for running the app
FROM gcr.io/distroless/cc-debian12

# Create app directory
WORKDIR /app

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/husky-rs /app/

# Expose the port (for example, 8080)
EXPOSE 8080

# Run the binary
CMD ["./husky-rs"]

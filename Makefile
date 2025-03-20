
aarch64-unknown-linux-gnu: 
	cross build --target=aarch64-unknown-linux-gnu --release
	docker build -f docker/Dockerfile.aarch64-unknown-linux-gnu -t 



import subprocess
import time
import re
import requests
import sys
import os
import signal

def main():
    print("Building and starting server...")
    # Start the server process
    # We use unbuffered output to catch the token immediately
    env = os.environ.copy()
    env["RUST_LOG"] = "info"

    process = subprocess.Popen(
        ["cargo", "run", "--bin", "ahutong-sdk", "--features", "server"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
        universal_newlines=True,
        env=env
    )

    base_url = "http://127.0.0.1:3000"
    token = None
    
    print("Waiting for server to start...")
    
    try:
        # Read stdout line by line to find the token
        start_time = time.time()
        while time.time() - start_time < 60:  # 60s timeout for compilation/startup
            line = process.stdout.readline()
            if not line:
                if process.poll() is not None:
                    print("Server process exited prematurely.")
                    print("Stderr:", process.stderr.read())
                    return
                continue
                
            print(f"[Server Output] {line.strip()}")
            
            # Match token pattern from main.rs: "Token: {}"
            match = re.search(r"Token: ([\w-]+)", line)
            if match:
                token = match.group(1)
                print(f"\n✅ Found Token: {token}")
                break
        
        if not token:
            print("❌ Failed to find token in server output.")
            return

        # Give it a moment to be fully ready
        time.sleep(1)

        # Test 1: Health Check (No Auth)
        print("\nTesting /health (No Auth)...")
        try:
            resp = requests.get(f"{base_url}/health")
            if resp.status_code == 200:
                print("✅ /health passed")
            else:
                print(f"❌ /health failed: {resp.status_code} {resp.text}")
        except Exception as e:
            print(f"❌ Connection failed: {e}")

        # Test 2: Protected Endpoint without Token
        print("\nTesting /exam (Without Token)...")
        resp = requests.get(f"{base_url}/exam")
        if resp.status_code == 401:
            print("✅ Auth check passed (401 Unauthorized received)")
        else:
            print(f"❌ Auth check failed: Expected 401, got {resp.status_code}")

        # Test 3: Protected Endpoint with Token
        print("\nTesting /exam (With Token)...")
        headers = {"X-AHUTONG-TOKEN": token}
        resp = requests.get(f"{base_url}/exam", headers=headers)
        
        # We expect 200 OK (even if empty list) or 500/400 if not logged in (crawler error)
        # But the server layer should accept the token.
        # Since we haven't logged in, the crawler might return an error, which is fine for this test 
        # as long as it's not 401.
        if resp.status_code != 401:
            print(f"✅ Token accepted (Status: {resp.status_code})")
            # If it's an error, it's likely "not logged in", which confirms the request went through to logic
            if resp.status_code != 200:
                print(f"   Response: {resp.text} (Expected since we didn't login)")
        else:
            print(f"❌ Token rejected: {resp.status_code}")

    except KeyboardInterrupt:
        print("\nTest interrupted.")
    finally:
        print("\nStopping server...")
        # On Windows, terminate might not kill the tree, but for this simple test it's okay
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
        print("Server stopped.")

if __name__ == "__main__":
    main()

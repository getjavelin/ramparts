#!/usr/bin/env python3
"""
Simple Ramparts Proxy Load Test

A lightweight load testing script that focuses on key performance metrics
without complex dependencies.
"""

import subprocess
import time
import json
import statistics
import threading
import requests
import os
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

class SimpleLoadTester:
    def __init__(self):
        self.proxy_process = None
        self.proxy_url = "http://localhost:8080"
        
    def log(self, message, prefix="ℹ️"):
        print(f"{prefix} {message}")
    
    def start_proxy(self):
        """Start the proxy in test mode"""
        self.log("🚀 Starting Ramparts proxy for load testing...")
        
        env = os.environ.copy()
        env.update({
            "JAVELIN_API_KEY": "test-mode",
            "RUST_LOG": "warn"  # Reduce logging overhead
        })
        
        self.proxy_process = subprocess.Popen([
            "cargo", "run", "--", "proxy", "127.0.0.1:8080"
        ], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        
        # Wait for startup
        time.sleep(6)
        
        # Verify proxy is running
        try:
            response = requests.get(f"{self.proxy_url}/health", timeout=5)
            if response.status_code == 200:
                self.log("✅ Proxy started successfully")
                return True
        except Exception as e:
            self.log(f"❌ Failed to start proxy: {e}")
            return False
        
        return False
    
    def stop_proxy(self):
        """Stop the proxy"""
        if self.proxy_process:
            self.proxy_process.terminate()
            self.proxy_process.wait()
            self.log("🛑 Proxy stopped")
    
    def make_request(self, endpoint, payload=None, method="GET"):
        """Make a single request and measure response time"""
        start_time = time.time()
        try:
            if method == "GET":
                response = requests.get(f"{self.proxy_url}{endpoint}", timeout=10)
            else:
                response = requests.post(f"{self.proxy_url}{endpoint}", 
                                       json=payload, timeout=10)
            
            response_time = time.time() - start_time
            return {
                'success': True,
                'response_time': response_time,
                'status_code': response.status_code,
                'error': None
            }
        except Exception as e:
            response_time = time.time() - start_time
            return {
                'success': False,
                'response_time': response_time,
                'status_code': 0,
                'error': str(e)
            }
    
    def concurrent_test(self, test_name, endpoint, payload=None, method="GET", 
                       num_threads=20, requests_per_thread=10):
        """Run concurrent requests test"""
        self.log(f"🧪 {test_name}: {num_threads} threads × {requests_per_thread} requests")
        
        results = []
        start_time = time.time()
        
        def worker():
            thread_results = []
            for _ in range(requests_per_thread):
                result = self.make_request(endpoint, payload, method)
                thread_results.append(result)
            return thread_results
        
        with ThreadPoolExecutor(max_workers=num_threads) as executor:
            futures = [executor.submit(worker) for _ in range(num_threads)]
            
            for future in as_completed(futures):
                results.extend(future.result())
        
        duration = time.time() - start_time
        
        # Calculate metrics
        total_requests = len(results)
        successful = sum(1 for r in results if r['success'])
        failed = total_requests - successful
        response_times = [r['response_time'] for r in results if r['success']]
        
        if response_times:
            avg_time = statistics.mean(response_times)
            min_time = min(response_times)
            max_time = max(response_times)
            p95_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 20 else max_time
        else:
            avg_time = min_time = max_time = p95_time = 0
        
        rps = total_requests / duration if duration > 0 else 0
        
        # Print results
        self.log(f"📊 {test_name} Results:")
        self.log(f"   Total Requests: {total_requests}")
        self.log(f"   Successful: {successful} ({successful/total_requests*100:.1f}%)")
        self.log(f"   Failed: {failed} ({failed/total_requests*100:.1f}%)")
        self.log(f"   Duration: {duration:.2f}s")
        self.log(f"   Requests/Second: {rps:.1f}")
        self.log(f"   Response Times:")
        self.log(f"     Average: {avg_time*1000:.1f}ms")
        self.log(f"     Min: {min_time*1000:.1f}ms")
        self.log(f"     Max: {max_time*1000:.1f}ms")
        self.log(f"     95th percentile: {p95_time*1000:.1f}ms")
        
        # Show errors if any
        errors = [r['error'] for r in results if not r['success'] and r['error']]
        if errors:
            unique_errors = list(set(errors))[:3]
            self.log(f"   Sample Errors: {unique_errors}")
        
        return {
            'test_name': test_name,
            'total_requests': total_requests,
            'successful': successful,
            'failed': failed,
            'rps': rps,
            'avg_response_time': avg_time,
            'p95_response_time': p95_time
        }
    
    def cache_test(self):
        """Test cache performance"""
        self.log("🧪 Cache Performance Test")
        
        payload = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "read_file",
                "arguments": {"path": "/tmp/cache_test.txt"}
            }
        }
        
        # First request (cache miss)
        first_result = self.make_request("/validate", payload, "POST")
        first_time = first_result['response_time']
        
        # Subsequent requests (cache hits)
        cache_times = []
        for _ in range(10):
            result = self.make_request("/validate", payload, "POST")
            if result['success']:
                cache_times.append(result['response_time'])
        
        if cache_times:
            avg_cache_time = statistics.mean(cache_times)
            speedup = first_time / avg_cache_time if avg_cache_time > 0 else 0
            
            self.log(f"📊 Cache Test Results:")
            self.log(f"   First request (miss): {first_time*1000:.1f}ms")
            self.log(f"   Cached requests (avg): {avg_cache_time*1000:.1f}ms")
            self.log(f"   Cache speedup: {speedup:.1f}x")
        else:
            self.log("❌ Cache test failed")
    
    def run_load_tests(self):
        """Run all load tests"""
        self.log("🚀 Ramparts Proxy Simple Load Test")
        self.log("=" * 50)
        
        if not self.start_proxy():
            self.log("❌ Cannot start proxy. Exiting.")
            return
        
        try:
            results = []
            
            # Test 1: Health endpoint load
            result1 = self.concurrent_test(
                "Health Endpoint Load",
                "/health",
                num_threads=25,
                requests_per_thread=20
            )
            results.append(result1)
            
            self.log("")
            
            # Test 2: Validation endpoint load
            safe_payload = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "read_file",
                    "arguments": {"path": "/tmp/safe.txt"}
                }
            }
            
            result2 = self.concurrent_test(
                "Validation Endpoint Load",
                "/validate",
                payload=safe_payload,
                method="POST",
                num_threads=20,
                requests_per_thread=15
            )
            results.append(result2)
            
            self.log("")
            
            # Test 3: Mixed safe/dangerous requests
            dangerous_payload = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "exec",
                    "arguments": {"command": "rm -rf /"}
                }
            }
            
            # Alternate between safe and dangerous
            def mixed_worker():
                thread_results = []
                for i in range(10):
                    payload = safe_payload if i % 2 == 0 else dangerous_payload
                    result = self.make_request("/validate", payload, "POST")
                    thread_results.append(result)
                return thread_results
            
            self.log("🧪 Mixed Request Load: Safe + Dangerous requests")
            results_mixed = []
            start_time = time.time()
            
            with ThreadPoolExecutor(max_workers=15) as executor:
                futures = [executor.submit(mixed_worker) for _ in range(15)]
                for future in as_completed(futures):
                    results_mixed.extend(future.result())
            
            duration = time.time() - start_time
            successful = sum(1 for r in results_mixed if r['success'])
            total = len(results_mixed)
            rps = total / duration if duration > 0 else 0
            
            self.log(f"📊 Mixed Request Results:")
            self.log(f"   Total: {total}, Success: {successful}, RPS: {rps:.1f}")
            
            self.log("")
            
            # Test 4: Cache performance
            self.cache_test()
            
            self.log("")
            
            # Summary
            self.log("📋 Load Test Summary")
            self.log("=" * 30)
            
            total_requests = sum(r['total_requests'] for r in results)
            total_successful = sum(r['successful'] for r in results)
            avg_rps = statistics.mean([r['rps'] for r in results])
            avg_response_time = statistics.mean([r['avg_response_time'] for r in results])
            
            self.log(f"Total Requests: {total_requests}")
            self.log(f"Success Rate: {total_successful/total_requests*100:.1f}%")
            self.log(f"Average RPS: {avg_rps:.1f}")
            self.log(f"Average Response Time: {avg_response_time*1000:.1f}ms")
            
            # Performance assessment
            if avg_rps > 300:
                self.log("🎉 Excellent performance!")
            elif avg_rps > 150:
                self.log("✅ Good performance")
            elif avg_rps > 75:
                self.log("⚠️  Moderate performance")
            else:
                self.log("❌ Performance needs improvement")
                
        except KeyboardInterrupt:
            self.log("⚠️  Load test interrupted")
        except Exception as e:
            self.log(f"❌ Load test failed: {e}")
        finally:
            self.stop_proxy()

def main():
    tester = SimpleLoadTester()
    tester.run_load_tests()

if __name__ == "__main__":
    main()

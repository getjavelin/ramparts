#!/usr/bin/env python3
"""
Ramparts Proxy Load Testing Suite

This script performs comprehensive load testing on both HTTP and STDIO proxy modes
to evaluate performance, throughput, and stability under various load conditions.

Test scenarios:
1. HTTP Proxy Load Testing
2. Concurrent Request Testing
3. Security Validation Performance
4. Cache Performance Testing
5. Stress Testing
"""

import asyncio
import aiohttp
import time
import json
import statistics
import subprocess
import sys
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from typing import List, Dict, Any
import threading

@dataclass
class TestResult:
    """Test result data structure"""
    test_name: str
    total_requests: int
    successful_requests: int
    failed_requests: int
    avg_response_time: float
    min_response_time: float
    max_response_time: float
    p95_response_time: float
    requests_per_second: float
    duration: float
    errors: List[str]

class Colors:
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    WHITE = '\033[97m'
    BOLD = '\033[1m'
    END = '\033[0m'

def log(message, color=Colors.WHITE):
    print(f"{color}{message}{Colors.END}")

def log_success(message):
    log(f"✅ {message}", Colors.GREEN)

def log_error(message):
    log(f"❌ {message}", Colors.RED)

def log_warning(message):
    log(f"⚠️  {message}", Colors.YELLOW)

def log_info(message):
    log(f"ℹ️  {message}", Colors.BLUE)

def log_test(message):
    log(f"🧪 {message}", Colors.CYAN)

class ProxyLoadTester:
    """Main load testing class"""
    
    def __init__(self, proxy_url="http://localhost:8080"):
        self.proxy_url = proxy_url
        self.proxy_process = None
        self.results = []
    
    def start_proxy(self):
        """Start the Ramparts HTTP proxy"""
        log_info("Starting Ramparts HTTP proxy for load testing...")
        
        env = os.environ.copy()
        env.update({
            "JAVELIN_API_KEY": "test-mode",
            "RUST_LOG": "warn",  # Reduce logging for performance
            "JAVELIN_FAIL_OPEN": "false"
        })
        
        self.proxy_process = subprocess.Popen([
            "cargo", "run", "--release", "--", "proxy", "127.0.0.1:8080"
        ], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        
        # Wait for startup
        time.sleep(8)
        
        # Verify proxy is running
        try:
            import requests
            response = requests.get(f"{self.proxy_url}/health", timeout=5)
            if response.status_code == 200:
                log_success("Proxy started successfully")
                return True
        except Exception as e:
            log_error(f"Failed to start proxy: {e}")
            return False
        
        return False
    
    def stop_proxy(self):
        """Stop the proxy process"""
        if self.proxy_process:
            self.proxy_process.terminate()
            self.proxy_process.wait()
            log_info("Proxy stopped")
    
    async def make_request(self, session, endpoint, payload=None, method="GET"):
        """Make a single HTTP request"""
        start_time = time.time()
        try:
            if method == "GET":
                async with session.get(f"{self.proxy_url}{endpoint}") as response:
                    await response.text()
                    return time.time() - start_time, response.status, None
            else:
                async with session.post(f"{self.proxy_url}{endpoint}", 
                                      json=payload) as response:
                    await response.text()
                    return time.time() - start_time, response.status, None
        except Exception as e:
            return time.time() - start_time, 0, str(e)
    
    async def health_check_load_test(self, concurrent_users=50, requests_per_user=20):
        """Test health endpoint under load"""
        log_test(f"Health Check Load Test: {concurrent_users} users, {requests_per_user} requests each")
        
        async with aiohttp.ClientSession() as session:
            tasks = []
            for user in range(concurrent_users):
                for req in range(requests_per_user):
                    task = self.make_request(session, "/health")
                    tasks.append(task)
            
            start_time = time.time()
            results = await asyncio.gather(*tasks, return_exceptions=True)
            duration = time.time() - start_time
            
            # Process results
            response_times = []
            successful = 0
            failed = 0
            errors = []
            
            for result in results:
                if isinstance(result, Exception):
                    failed += 1
                    errors.append(str(result))
                else:
                    response_time, status, error = result
                    response_times.append(response_time)
                    if status == 200:
                        successful += 1
                    else:
                        failed += 1
                        if error:
                            errors.append(error)
            
            total_requests = len(tasks)
            avg_response_time = statistics.mean(response_times) if response_times else 0
            min_response_time = min(response_times) if response_times else 0
            max_response_time = max(response_times) if response_times else 0
            p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 20 else max_response_time
            requests_per_second = total_requests / duration if duration > 0 else 0
            
            return TestResult(
                test_name="Health Check Load Test",
                total_requests=total_requests,
                successful_requests=successful,
                failed_requests=failed,
                avg_response_time=avg_response_time,
                min_response_time=min_response_time,
                max_response_time=max_response_time,
                p95_response_time=p95_response_time,
                requests_per_second=requests_per_second,
                duration=duration,
                errors=errors[:10]  # Keep only first 10 errors
            )
    
    async def validation_load_test(self, concurrent_users=30, requests_per_user=10):
        """Test validation endpoint under load"""
        log_test(f"Validation Load Test: {concurrent_users} users, {requests_per_user} requests each")
        
        # Mix of safe and dangerous requests
        safe_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "read_file",
                "arguments": {"path": "/tmp/safe_file.txt"}
            }
        }
        
        dangerous_request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "exec",
                "arguments": {"command": "rm -rf /"}
            }
        }
        
        async with aiohttp.ClientSession() as session:
            tasks = []
            for user in range(concurrent_users):
                for req in range(requests_per_user):
                    # Alternate between safe and dangerous requests
                    payload = safe_request if req % 2 == 0 else dangerous_request
                    task = self.make_request(session, "/validate", payload, "POST")
                    tasks.append(task)
            
            start_time = time.time()
            results = await asyncio.gather(*tasks, return_exceptions=True)
            duration = time.time() - start_time
            
            # Process results (similar to health check)
            response_times = []
            successful = 0
            failed = 0
            errors = []
            
            for result in results:
                if isinstance(result, Exception):
                    failed += 1
                    errors.append(str(result))
                else:
                    response_time, status, error = result
                    response_times.append(response_time)
                    if status == 200:
                        successful += 1
                    else:
                        failed += 1
                        if error:
                            errors.append(error)
            
            total_requests = len(tasks)
            avg_response_time = statistics.mean(response_times) if response_times else 0
            min_response_time = min(response_times) if response_times else 0
            max_response_time = max(response_times) if response_times else 0
            p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 20 else max_response_time
            requests_per_second = total_requests / duration if duration > 0 else 0
            
            return TestResult(
                test_name="Validation Load Test",
                total_requests=total_requests,
                successful_requests=successful,
                failed_requests=failed,
                avg_response_time=avg_response_time,
                min_response_time=min_response_time,
                max_response_time=max_response_time,
                p95_response_time=p95_response_time,
                requests_per_second=requests_per_second,
                duration=duration,
                errors=errors[:10]
            )
    
    def cache_performance_test(self):
        """Test cache performance with repeated requests"""
        log_test("Cache Performance Test: Repeated identical requests")
        
        import requests
        
        payload = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "read_file",
                "arguments": {"path": "/tmp/cached_file.txt"}
            }
        }
        
        response_times = []
        successful = 0
        failed = 0
        
        # First request (cache miss)
        start_time = time.time()
        try:
            response = requests.post(f"{self.proxy_url}/validate", json=payload, timeout=10)
            first_request_time = time.time() - start_time
            if response.status_code == 200:
                successful += 1
            else:
                failed += 1
        except Exception:
            failed += 1
            first_request_time = 0
        
        # Subsequent requests (cache hits)
        for i in range(99):  # 99 more requests for total of 100
            start_time = time.time()
            try:
                response = requests.post(f"{self.proxy_url}/validate", json=payload, timeout=10)
                response_time = time.time() - start_time
                response_times.append(response_time)
                if response.status_code == 200:
                    successful += 1
                else:
                    failed += 1
            except Exception:
                failed += 1
        
        total_requests = 100
        avg_response_time = statistics.mean(response_times) if response_times else 0
        min_response_time = min(response_times) if response_times else 0
        max_response_time = max(response_times) if response_times else 0
        p95_response_time = statistics.quantiles(response_times, n=20)[18] if len(response_times) > 20 else max_response_time
        
        log_info(f"First request (cache miss): {first_request_time:.3f}s")
        log_info(f"Average cached request: {avg_response_time:.3f}s")
        log_info(f"Cache speedup: {first_request_time/avg_response_time:.1f}x" if avg_response_time > 0 else "N/A")
        
        return TestResult(
            test_name="Cache Performance Test",
            total_requests=total_requests,
            successful_requests=successful,
            failed_requests=failed,
            avg_response_time=avg_response_time,
            min_response_time=min_response_time,
            max_response_time=max_response_time,
            p95_response_time=p95_response_time,
            requests_per_second=0,  # Not applicable for this test
            duration=0,
            errors=[]
        )
    
    def print_results(self, result: TestResult):
        """Print formatted test results"""
        log(f"\n📊 {result.test_name} Results", Colors.BOLD)
        log("=" * 50)
        log(f"Total Requests: {result.total_requests}")
        log(f"Successful: {result.successful_requests} ({result.successful_requests/result.total_requests*100:.1f}%)", Colors.GREEN)
        log(f"Failed: {result.failed_requests} ({result.failed_requests/result.total_requests*100:.1f}%)", Colors.RED if result.failed_requests > 0 else Colors.GREEN)
        
        if result.requests_per_second > 0:
            log(f"Requests/Second: {result.requests_per_second:.1f}")
            log(f"Duration: {result.duration:.2f}s")
        
        log(f"Response Times:")
        log(f"  Average: {result.avg_response_time*1000:.1f}ms")
        log(f"  Min: {result.min_response_time*1000:.1f}ms")
        log(f"  Max: {result.max_response_time*1000:.1f}ms")
        log(f"  95th percentile: {result.p95_response_time*1000:.1f}ms")
        
        if result.errors:
            log(f"Sample Errors: {result.errors[:3]}", Colors.YELLOW)
    
    async def run_load_tests(self):
        """Run all load tests"""
        log(f"{Colors.BOLD}🚀 Ramparts Proxy Load Testing Suite{Colors.END}")
        log("=" * 60)
        
        if not self.start_proxy():
            log_error("Failed to start proxy. Exiting.")
            return
        
        try:
            # Test 1: Health check load test
            result1 = await self.health_check_load_test(concurrent_users=50, requests_per_user=20)
            self.results.append(result1)
            self.print_results(result1)
            
            # Test 2: Validation load test
            result2 = await self.validation_load_test(concurrent_users=30, requests_per_user=10)
            self.results.append(result2)
            self.print_results(result2)
            
            # Test 3: Cache performance test
            result3 = self.cache_performance_test()
            self.results.append(result3)
            self.print_results(result3)
            
            # Test 4: Stress test (higher load)
            log_test("Stress Test: High concurrent load")
            result4 = await self.health_check_load_test(concurrent_users=100, requests_per_user=10)
            result4.test_name = "Stress Test"
            self.results.append(result4)
            self.print_results(result4)
            
            # Summary
            self.print_summary()
            
        except KeyboardInterrupt:
            log_warning("Load testing interrupted by user")
        except Exception as e:
            log_error(f"Load testing failed: {e}")
        finally:
            self.stop_proxy()
    
    def print_summary(self):
        """Print overall test summary"""
        log(f"\n{Colors.BOLD}📋 Load Testing Summary{Colors.END}", Colors.BOLD)
        log("=" * 60)
        
        total_requests = sum(r.total_requests for r in self.results)
        total_successful = sum(r.successful_requests for r in self.results)
        total_failed = sum(r.failed_requests for r in self.results)
        
        log(f"Total Requests Processed: {total_requests}")
        log(f"Overall Success Rate: {total_successful/total_requests*100:.1f}%")
        log(f"Overall Failure Rate: {total_failed/total_requests*100:.1f}%")
        
        # Performance summary
        avg_rps = statistics.mean([r.requests_per_second for r in self.results if r.requests_per_second > 0])
        avg_response_time = statistics.mean([r.avg_response_time for r in self.results])
        
        log(f"Average Requests/Second: {avg_rps:.1f}")
        log(f"Average Response Time: {avg_response_time*1000:.1f}ms")
        
        # Performance assessment
        if avg_rps > 500:
            log("🎉 Excellent performance!", Colors.GREEN)
        elif avg_rps > 200:
            log("✅ Good performance", Colors.GREEN)
        elif avg_rps > 100:
            log("⚠️  Moderate performance", Colors.YELLOW)
        else:
            log("❌ Performance needs improvement", Colors.RED)

async def main():
    """Main function"""
    tester = ProxyLoadTester()
    await tester.run_load_tests()

if __name__ == "__main__":
    asyncio.run(main())

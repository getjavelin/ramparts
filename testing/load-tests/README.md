# Ramparts Proxy Load Testing

This directory contains load testing scripts for evaluating Ramparts proxy performance under various conditions.

## 📁 Test Scripts

### 🧪 **simple_load_test.py**
**Purpose**: Basic load testing with standard scenarios  
**Features**:
- Health endpoint load testing
- Validation endpoint testing
- Mixed safe/dangerous request testing
- Cache performance evaluation
- Simple threading-based approach

**Usage**:
```bash
python3 simple_load_test.py
```

**Test Scenarios**:
- 25 threads × 20 requests (Health endpoint)
- 20 threads × 15 requests (Validation endpoint)
- 15 threads × 10 requests (Mixed requests)
- Cache performance test

### 🔥 **stress_test.py**
**Purpose**: High-intensity stress testing to find performance limits  
**Features**:
- Escalating load testing (10-300 threads)
- Sustained load testing (20+ seconds)
- Memory stress testing (large payloads)
- Performance limit detection

**Usage**:
```bash
python3 stress_test.py
```

**Test Scenarios**:
- Escalating: 10, 25, 50, 75, 100, 150, 200, 300 threads
- Sustained: 50 threads for 20 seconds
- Memory: 20 threads × 5 requests with 10KB payloads

### ⚡ **load_test.py**
**Purpose**: Comprehensive async load testing suite  
**Features**:
- Async/await based testing
- Advanced metrics collection
- Multiple concurrent test scenarios
- Detailed performance analysis

**Usage**:
```bash
python3 load_test.py
```

**Requirements**: `aiohttp` library
```bash
pip install aiohttp
```

## 📊 **Test Results**

See [Load Test Results](../../docs/proxy/LOAD_TEST_RESULTS.md) for comprehensive performance analysis.

### **Key Performance Metrics**
- **Peak Throughput**: 1,330 RPS
- **Concurrent Users**: 300 threads successfully handled
- **Average Response Time**: 15.3ms
- **Success Rate**: 100% under normal load
- **95th Percentile**: <35ms

## 🚀 **Quick Start**

### **Run Basic Load Test**
```bash
cd testing/load-tests
python3 simple_load_test.py
```

### **Run Stress Test**
```bash
cd testing/load-tests
python3 stress_test.py
```

### **Expected Output**
```
🚀 Ramparts Proxy Simple Load Test
==================================================
ℹ️ 🚀 Starting Ramparts proxy for load testing...
ℹ️ ✅ Proxy started successfully
ℹ️ 🧪 Health Endpoint Load: 25 threads × 20 requests
ℹ️ 📊 Health Endpoint Load Results:
ℹ️    Total Requests: 500
ℹ️    Successful: 500 (100.0%)
ℹ️    Requests/Second: 1295.2
ℹ️    Response Times: Average: 16.9ms
ℹ️ 🎉 Excellent performance!
```

## 🔧 **Configuration**

### **Environment Variables**
The test scripts automatically configure:
- `JAVELIN_API_KEY=test-mode` - Bypass Javelin API for testing
- `RUST_LOG=warn` or `RUST_LOG=error` - Reduce logging overhead

### **Test Parameters**
You can modify test parameters in each script:

**simple_load_test.py**:
```python
# Health endpoint test
num_threads=25, requests_per_thread=20

# Validation endpoint test  
num_threads=20, requests_per_thread=15
```

**stress_test.py**:
```python
# Escalating test thread counts
thread_counts = [10, 25, 50, 75, 100, 150, 200, 300]

# Sustained test duration
duration_seconds = 20
```

## 📈 **Performance Benchmarks**

### **Excellent Performance** (>300 RPS)
- ✅ Ready for production
- ✅ Can handle high-traffic scenarios
- ✅ Suitable for enterprise deployment

### **Good Performance** (150-300 RPS)
- ✅ Suitable for most production workloads
- ⚠️ Monitor under peak load

### **Moderate Performance** (75-150 RPS)
- ⚠️ Consider optimization
- ⚠️ May need horizontal scaling

### **Needs Improvement** (<75 RPS)
- ❌ Investigate performance issues
- ❌ Not suitable for production without optimization

## 🛡️ **Security Testing**

All load tests include security validation testing:

### **Safe Requests** (Should be allowed)
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "read_file",
    "arguments": {"path": "/tmp/safe.txt"}
  }
}
```

### **Dangerous Requests** (Should be blocked)
```json
{
  "jsonrpc": "2.0", 
  "method": "tools/call",
  "params": {
    "name": "exec",
    "arguments": {"command": "rm -rf /"}
  }
}
```

## 🔍 **Troubleshooting**

### **Common Issues**

**Proxy fails to start**:
- Ensure port 8080 is available
- Check if another proxy instance is running
- Verify Rust/Cargo installation

**Connection refused errors**:
- Increase startup wait time in test scripts
- Check proxy logs for startup errors
- Verify proxy is listening on correct port

**Low performance**:
- Use `--release` build for better performance
- Reduce logging level (`RUST_LOG=error`)
- Check system resource availability

### **Performance Tuning**

**For higher throughput**:
1. Use release build: `cargo run --release`
2. Increase system limits: `ulimit -n 4096`
3. Tune OS network parameters
4. Consider multiple proxy instances

**For lower latency**:
1. Reduce thread count in tests
2. Use connection pooling
3. Enable HTTP/2 if supported
4. Optimize network configuration

## 📚 **Additional Resources**

- [Proxy Documentation](../../docs/proxy/README.md)
- [Configuration Guide](../../docs/proxy/configuration.md)
- [E2E Test Results](../../docs/proxy/E2E_TEST_RESULTS.md)
- [Load Test Results](../../docs/proxy/LOAD_TEST_RESULTS.md)

## 🎯 **Next Steps**

1. **Baseline Testing**: Run simple_load_test.py to establish baseline
2. **Stress Testing**: Run stress_test.py to find performance limits
3. **Production Planning**: Use results to plan deployment capacity
4. **Monitoring Setup**: Implement monitoring based on test metrics
5. **Optimization**: Tune configuration based on test results

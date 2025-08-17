using System;
using System.Collections.Generic;

namespace SensorBridge.Tests
{
    public class TestSummary
    {
        public DateTime TestStartTime { get; set; }
        public DateTime TestEndTime { get; set; }
        public TimeSpan TotalDuration { get; set; }
        public int TotalTests { get; set; }
        public int PassedTests { get; set; }
        public int FailedTests { get; set; }
        public double SuccessRate { get; set; }
        public bool IsAdministrator { get; set; }
        public List<TestResult> TestResults { get; set; } = new List<TestResult>();
        public string? ReportPath { get; set; }
    }

    public class TestResult
    {
        public string TestName { get; set; } = string.Empty;
        public bool Success { get; set; }
        public string Message { get; set; } = string.Empty;
        public DateTime StartTime { get; set; }
        public DateTime EndTime { get; set; }
        public TimeSpan Duration { get; set; }
        public object? Details { get; set; }
        public string? ErrorDetails { get; set; }
    }
}

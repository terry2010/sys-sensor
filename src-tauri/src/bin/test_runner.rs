// Tauri 后端测试运行器可执行文件
use sys_sensor_lib::test_runner::TestRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_runner = TestRunner::new();
    
    match test_runner.run_all_tests().await {
        Ok(summary) => {
            println!("测试完成！");
            println!("总测试数: {}", summary.total_tests);
            println!("通过: {}", summary.passed_tests);
            println!("失败: {}", summary.failed_tests);
            println!("成功率: {:.1}%", summary.success_rate);
            println!("报告路径: {}", summary.report_path);
            
            if summary.failed_tests > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("测试运行失败: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

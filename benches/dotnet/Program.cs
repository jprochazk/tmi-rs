using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Jobs;
using BenchmarkDotNet.Running;
using BenchmarkDotNet.Toolchains.InProcess.NoEmit;
using libs_comparison;


ManualConfig config = DefaultConfig.Instance
    .AddJob(Job
         .MediumRun
         .WithLaunchCount(1)
         .WithWarmupCount(5)
         .WithIterationCount(5)
         .WithToolchain(InProcessNoEmitToolchain.Instance));
BenchmarkRunner.Run<ComparisonBenchmarks>(config);

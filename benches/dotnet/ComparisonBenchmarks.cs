using System.Text;
using BenchmarkDotNet.Attributes;
using forsen;

namespace libs_comparison;
[MemoryDiagnoser(true)]
public class ComparisonBenchmarks
{
    private readonly List<ReadOnlyMemory<byte>> dataLines = new();
    private readonly List<string> stringLines = new();

    [GlobalSetup]
    public void AddData()
    {
        string[] lines = File.ReadLines("data.txt").Take(1000).ToArray();

        foreach (string line in lines)
        {
            stringLines.Add(line);
            dataLines.Add(Encoding.UTF8.GetBytes(line));
        }

        Console.WriteLine($"Added {dataLines.Count} lines");
    }

    [Benchmark]
    public void MiniTwitchParse()
    {
        foreach (ReadOnlyMemory<byte> item in dataLines)
        {
            MiniTwitch.Process(item);
        }
    }

    /* [Benchmark]
    public void TwitchLibParse()
    {
        foreach (string item in stringLines)
        {
            TwitchLib.HandleIrcMessage(TwitchLib.ParseIrcMessage(item));
        }
    } */

    [GlobalCleanup]
    public void Cleanup()
    {
        Console.WriteLine($"Clearing {dataLines.Count} lines");
        dataLines.Clear();
        stringLines.Clear();
    }
}

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();
app.MapGet("/api/greeting", () => new { message = Consumer.Greeting.Message });
app.Run();
public partial class Program { }

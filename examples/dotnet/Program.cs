var builder = WebApplication.CreateBuilder(args);
builder.Services.AddCors(options =>
    options.AddDefaultPolicy(policy =>
        policy.WithOrigins("http://127.0.0.1:5186")
            .AllowAnyHeader()
            .AllowAnyMethod()));

var app = builder.Build();
app.UseCors();
app.MapGet("/api/greeting", (HttpRequest request) =>
    GreetingAuthorization.IsAuthorized(request.Headers.Authorization.ToString())
        ? Results.Ok(new { message = "Hello policy" })
        : Results.Unauthorized());

app.Run();

public static class GreetingAuthorization
{
    public static bool IsAuthorized(string? authorization) =>
        string.Equals(authorization, "Bearer acceptance-token", StringComparison.Ordinal);
}

public partial class Program
{
}

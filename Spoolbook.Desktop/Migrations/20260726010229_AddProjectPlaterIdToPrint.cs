using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Spoolbook.Desktop.Migrations
{
    /// <inheritdoc />
    public partial class AddProjectPlaterIdToPrint : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "ProjectPlaterId",
                table: "Prints",
                type: "TEXT",
                nullable: true);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "ProjectPlaterId",
                table: "Prints");
        }
    }
}

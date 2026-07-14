using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Spoolbook.Desktop.Migrations
{
    /// <inheritdoc />
    public partial class AddPrinterEntity : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "Printer",
                table: "Prints");

            migrationBuilder.AddColumn<int>(
                name: "PrinterId",
                table: "Prints",
                type: "INTEGER",
                nullable: false,
                defaultValue: 0);

            migrationBuilder.CreateTable(
                name: "Printers",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    Name = table.Column<string>(type: "TEXT", nullable: false),
                    Model = table.Column<string>(type: "TEXT", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Printers", x => x.Id);
                });

            migrationBuilder.CreateIndex(
                name: "IX_Prints_PrinterId",
                table: "Prints",
                column: "PrinterId");

            migrationBuilder.CreateIndex(
                name: "IX_Printers_Name",
                table: "Printers",
                column: "Name",
                unique: true);

            migrationBuilder.AddForeignKey(
                name: "FK_Prints_Printers_PrinterId",
                table: "Prints",
                column: "PrinterId",
                principalTable: "Printers",
                principalColumn: "Id",
                onDelete: ReferentialAction.Restrict);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropForeignKey(
                name: "FK_Prints_Printers_PrinterId",
                table: "Prints");

            migrationBuilder.DropTable(
                name: "Printers");

            migrationBuilder.DropIndex(
                name: "IX_Prints_PrinterId",
                table: "Prints");

            migrationBuilder.DropColumn(
                name: "PrinterId",
                table: "Prints");

            migrationBuilder.AddColumn<string>(
                name: "Printer",
                table: "Prints",
                type: "TEXT",
                nullable: false,
                defaultValue: "");
        }
    }
}

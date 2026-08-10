using System;
using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Spoolbook.Desktop.Migrations
{
    /// <inheritdoc />
    public partial class AddPrinterTelemetry : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "AccessCode",
                table: "Printers",
                type: "TEXT",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "IpAddress",
                table: "Printers",
                type: "TEXT",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "SerialNumber",
                table: "Printers",
                type: "TEXT",
                nullable: true);

            migrationBuilder.CreateTable(
                name: "PrinterJobs",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    PrinterId = table.Column<int>(type: "INTEGER", nullable: false),
                    ExternalJobId = table.Column<string>(type: "TEXT", nullable: false),
                    StartedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    EndedAt = table.Column<DateTime>(type: "TEXT", nullable: true),
                    PrintId = table.Column<int>(type: "INTEGER", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_PrinterJobs", x => x.Id);
                    table.ForeignKey(
                        name: "FK_PrinterJobs_Printers_PrinterId",
                        column: x => x.PrinterId,
                        principalTable: "Printers",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                    table.ForeignKey(
                        name: "FK_PrinterJobs_Prints_PrintId",
                        column: x => x.PrintId,
                        principalTable: "Prints",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.SetNull);
                });

            migrationBuilder.CreateTable(
                name: "PrinterReadings",
                columns: table => new
                {
                    Id = table.Column<int>(type: "INTEGER", nullable: false)
                        .Annotation("Sqlite:Autoincrement", true),
                    PrinterJobId = table.Column<int>(type: "INTEGER", nullable: false),
                    RecordedAt = table.Column<DateTime>(type: "TEXT", nullable: false),
                    NozzleTempC = table.Column<decimal>(type: "TEXT", nullable: true),
                    BedTempC = table.Column<decimal>(type: "TEXT", nullable: true),
                    ChamberTempC = table.Column<decimal>(type: "TEXT", nullable: true),
                    AmsSlot = table.Column<string>(type: "TEXT", nullable: true),
                    ProgressPct = table.Column<int>(type: "INTEGER", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_PrinterReadings", x => x.Id);
                    table.ForeignKey(
                        name: "FK_PrinterReadings_PrinterJobs_PrinterJobId",
                        column: x => x.PrinterJobId,
                        principalTable: "PrinterJobs",
                        principalColumn: "Id",
                        onDelete: ReferentialAction.Cascade);
                });

            migrationBuilder.CreateIndex(
                name: "IX_PrinterJobs_PrinterId",
                table: "PrinterJobs",
                column: "PrinterId");

            migrationBuilder.CreateIndex(
                name: "IX_PrinterJobs_PrintId",
                table: "PrinterJobs",
                column: "PrintId");

            migrationBuilder.CreateIndex(
                name: "IX_PrinterReadings_PrinterJobId",
                table: "PrinterReadings",
                column: "PrinterJobId");
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "PrinterReadings");

            migrationBuilder.DropTable(
                name: "PrinterJobs");

            migrationBuilder.DropColumn(
                name: "AccessCode",
                table: "Printers");

            migrationBuilder.DropColumn(
                name: "IpAddress",
                table: "Printers");

            migrationBuilder.DropColumn(
                name: "SerialNumber",
                table: "Printers");
        }
    }
}

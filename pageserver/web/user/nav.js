function DeleteAjax(requestUrl, redirectUrl) {
  $.ajax({
    type: "DELETE",
    url: requestUrl,
    success: function(data){
	window.location.href = redirectUrl;
    },
    error: function(resp){
	 alert(resp.responseText);
    },
    contentType : "text/plain; charset=utf-8"
  });
};

function DeleteCompany(company) {
  var requestUrl = "/companies/" + company;
  var redirectUrl = "/companies";
  DeleteAjax(requestUrl, redirectUrl);
};

function ShowDeleteCompanyModal(company) {
  $('#deleteCompanyModalText').text("The company " + company +
    " will be deleted with all its wells, users, interpretations. Are you sure?");
  $("#deleteCompanyModalActionButton").attr("onclick","DeleteCompany('" + company + "');");
  $('#deleteCompanyModal').modal('toggle');
};

function DeleteWell(company, well) {
  var requestUrl = "/companies/" + company + "/wells/" + well;
  var redirectUrl = "/companies/" + company + "/wells";
  DeleteAjax(requestUrl, redirectUrl);
};

function ShowDeleteWellModal(company, well) {
  $('#deleteWellModalText').text("The well " + well +
    " will be deleted with all its data and interpretations. Are you sure?");
  $("#deleteWellModalActionButton").attr("onclick","DeleteWell('" +
    company + "','" + well + "');");
  $('#deleteWellModal').modal('toggle');
};

function DeleteTag(company, well, tag) {
  var baseUrl = "/companies/" + company + "/wells/" + well + "/tags";
  var requestUrl = baseUrl + "/" + tag;
  var redirectUrl = baseUrl;
  DeleteAjax(requestUrl, redirectUrl);
};

function ShowDeleteTagModal(company, well, tag) {
  $('#deleteTagModalText').text("The tag #" + tag +
    " will be deleted with all its data. Are you sure?");
  $("#deleteTagModalActionButton").attr("onclick","DeleteTag('" +
    company + "','" + well + "'," + tag + ");");
  $('#deleteTagModal').modal('toggle');
};

function DeleteUser(company, user) {
  var baseUrl = "/companies/" + company + "/users";
  var requestUrl = baseUrl + "/" + user;
  var redirectUrl = baseUrl;
  DeleteAjax(requestUrl, redirectUrl);
};

function ShowDeleteUserModal(company, user) {
  $('#deleteUserModalText').text("The user " + user +
    " will be deleted with all its activity. Are you sure?");
  $("#deleteUserModalActionButton").attr("onclick","DeleteUser('" +
    company + "','" + user + "');");
  $('#deleteUserModal').modal('toggle');
};

function ChangeCycleStatus(company, well, cycle, newStatus) {
  var requestUrl = "/companies/" + company + "/wells/" +
      well + "/cycles/" + cycle + "/status";

  var form = {};
  form["newStatus"] = newStatus;

  $.ajax({
    type: "POST",
    url: requestUrl,
    data: form,
    success: function(data){ 
    },
    error: function(resp){
         alert(resp.responseText);
    },
    contentType : "application/x-www-form-urlencoded"
  });
}

function DeleteCycle(company, well, cycle) {
  var requestUrl = "/companies/" + company + "/wells/" +
      well + "/cycles/" + cycle.toString();
  $.ajax({
    type: "DELETE",
    url: requestUrl,
    success: function(data){
      var redirectUrl = "/companies/" + company + "/wells/" + well + "/cycles";
      window.location.href = redirectUrl;
    },
    error: function(resp){
      $('#warningModalText').text("Server response: failed to delete this cycle.");
      $('#warningModal').modal('toggle');
      return;
    },
    contentType : "text/plain"
  });
};

